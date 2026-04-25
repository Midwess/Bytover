use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::{AppModel, BitBridge};
use crate::entities::capabilities::{UserCapabilities, EXPECTED_CAPABILITIES_VERSION};
use crate::entities::user::User;
use chrono::{DateTime, Duration, Utc};
use core_services::utils::string::StringExt;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

use crate::app::modules::AppModule;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::AppEvent;
use crate::CoreOperation;

pub const CAPABILITIES_REFRESH_DEBOUNCE_SECS: i64 = 5;

pub struct AuthenticationModule;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationModel {
    pub user: Option<User>,
    pub capabilities: Option<UserCapabilities>,
    pub is_already_feedback: bool,
    #[serde(skip)]
    pub is_refreshing_capabilities: bool,
    #[serde(skip)]
    pub last_capabilities_refresh_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationViewModel {
    pub user: Option<User>,
    pub capabilities: Option<UserCapabilities>,
    pub is_already_feedback: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthenticationEvent {
    Authenticate,
    SignOut,
    OnRedirected { url: String },
    Authorized { user: User },
    UnAuthorized,
    Feedback { message: Option<String>, email: String },
    RefreshCapabilities,
    #[serde(skip)]
    RefreshCapabilitiesStarted,
    #[serde(skip)]
    RefreshCapabilitiesFailed(String),
    #[serde(skip)]
    CapabilitiesLoaded(UserCapabilities),
}

impl AppModule<BitBridge> for AuthenticationModule {
    type Event = AuthenticationEvent;
    type ViewModel = AuthenticationViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities,
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            AuthenticationEvent::Authenticate => Command::handle_result(|ctx| async move {
                ctx.app().authenticate().await;
                Ok(())
            }),
            AuthenticationEvent::SignOut => {
                model.authentication.user.take();
                model.authentication.capabilities.take();
                model.authentication.is_refreshing_capabilities = false;
                model.authentication.last_capabilities_refresh_at = None;
                Command::new(|it| async move {
                    let _ = it.app().run(P2POperation::stop()).await;
                })
                .then_render()
            }
            AuthenticationEvent::OnRedirected { url } => {
                if model.authentication.user.is_some() {
                    log::info!("User is already authorized, skipping...");
                    return Command::done();
                }

                Command::handle_result(|ctx| async move {
                    ctx.app().authorize(url).await?;
                    Ok(())
                })
            }
            AuthenticationEvent::Authorized { user } => {
                model.authentication.user.replace(user);

                let request_refresh = Command::new(|it| async move {
                    it.app().notify_event(AppEvent::Authentication(AuthenticationEvent::RefreshCapabilities));
                });

                Command::all(vec![Command::render(), request_refresh])
            }
            AuthenticationEvent::RefreshCapabilities => {
                if model.authentication.user.is_none() {
                    return Command::done();
                }
                if model.authentication.is_refreshing_capabilities {
                    return Command::done();
                }
                if let Some(last) = model.authentication.last_capabilities_refresh_at {
                    if Utc::now().signed_duration_since(last) < Duration::seconds(CAPABILITIES_REFRESH_DEBOUNCE_SECS) {
                        return Command::done();
                    }
                }

                model.authentication.is_refreshing_capabilities = true;

                Command::new(|it| async move {
                    it.app().refresh_capabilities().await;
                })
            }
            AuthenticationEvent::RefreshCapabilitiesStarted => {
                model.authentication.is_refreshing_capabilities = true;
                Command::done()
            }
            AuthenticationEvent::RefreshCapabilitiesFailed(message) => {
                model.authentication.is_refreshing_capabilities = false;
                log::warn!("Failed to refresh user capabilities: {message}");
                Command::done()
            }
            AuthenticationEvent::CapabilitiesLoaded(caps) => {
                if caps.capabilities_version > EXPECTED_CAPABILITIES_VERSION {
                    log::error!(
                        "Server sent capabilities_version={} but client expects <= {}; refusing to apply. Update the client.",
                        caps.capabilities_version,
                        EXPECTED_CAPABILITIES_VERSION
                    );
                    return Command::done();
                }
                if caps.capabilities_version < EXPECTED_CAPABILITIES_VERSION {
                    log::warn!(
                        "Server sent capabilities_version={} older than client expects ({}); applying with defaults where missing.",
                        caps.capabilities_version,
                        EXPECTED_CAPABILITIES_VERSION
                    );
                }

                let limit = caps.shelf_limit();
                model.authentication.capabilities.replace(caps);
                model.authentication.is_refreshing_capabilities = false;
                model.authentication.last_capabilities_refresh_at = Some(Utc::now());

                let cleanup = match limit {
                    Some(limit) => Command::handle_result(move |it| async move {
                        it.app().enforce_shelf_limit(limit as usize).await
                    }),
                    None => Command::done(),
                };

                Command::all(vec![Command::render(), cleanup])
            }
            AuthenticationEvent::UnAuthorized => Command::render(),
            AuthenticationEvent::Feedback { email, message } => {
                if let Some(message) = message.as_ref() {
                    if message.len() > 4024 {
                        return Command::new(|it| async move {
                            it.app().run(DialogOperation::toast("Message is too long".to_string())).await;
                        });
                    }
                }

                if !email.is_email() {
                    return Command::new(|it| async move {
                        it.app().run(DialogOperation::toast("Invalid email format".to_string())).await;
                    });
                }

                model.authentication.is_already_feedback = true;
                Command::handle_result(|ctx| async move {
                    ctx.app().notify_shell(CoreOperation::Render);
                    ctx.app().run(RpcOperation::feedback(email, message.unwrap_or_default())).await
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        AuthenticationViewModel {
            user: model.authentication.user.clone(),
            capabilities: model.authentication.capabilities.clone(),
            is_already_feedback: model.authentication.is_already_feedback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::operations::CoreOperation;
    use crate::app::operations::rpc::RpcOperation;
    use crate::app::{AppEvent, AppOperation, BitBridge};
    use crate::entities::capabilities::{Plan, PresentationLimits, UserCapabilities};
    use crux_core::App;

    fn signed_in_model() -> AppModel {
        let mut model = AppModel::default();
        model.authentication.user = Some(User {
            id: 1,
            email: "test@example.com".to_owned(),
            name: "Test".to_owned(),
            avatar: String::new(),
        });
        model
    }

    fn paid_capabilities() -> UserCapabilities {
        UserCapabilities {
            plan: Plan::Paid,
            presentation: PresentationLimits { max_visible_shelves: 0 },
            ..UserCapabilities::free_defaults()
        }
    }

    fn collect_effects(command: &mut crate::app::AppCommand) -> Vec<CoreOperation> {
        command
            .effects()
            .map(|effect| {
                let AppOperation::Operation(request) = effect;
                let (op, _) = request.split();
                op
            })
            .collect()
    }

    #[test]
    fn refresh_capabilities_event_dispatches_get_capabilities_when_idle() {
        let app = BitBridge::default();
        let mut model = signed_in_model();

        let mut command = app.update(AppEvent::Authentication(AuthenticationEvent::RefreshCapabilities), &mut model, &());
        let _ = collect_effects(&mut command);

        assert!(model.authentication.is_refreshing_capabilities);
        assert!(model.authentication.last_capabilities_refresh_at.is_none());
    }

    #[test]
    fn refresh_capabilities_event_skips_when_already_in_flight() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.authentication.is_refreshing_capabilities = true;

        let mut command = app.update(AppEvent::Authentication(AuthenticationEvent::RefreshCapabilities), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(model.authentication.is_refreshing_capabilities);
        assert!(
            !effects.iter().any(|op| matches!(op, CoreOperation::Rpc(RpcOperation::GetCapabilities))),
            "no GetCapabilities effect should be emitted while a refresh is in flight"
        );
    }

    #[test]
    fn refresh_capabilities_event_skips_when_within_debounce_window() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.authentication.last_capabilities_refresh_at = Some(Utc::now());

        let mut command = app.update(AppEvent::Authentication(AuthenticationEvent::RefreshCapabilities), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(!model.authentication.is_refreshing_capabilities);
        assert!(
            !effects.iter().any(|op| matches!(op, CoreOperation::Rpc(RpcOperation::GetCapabilities))),
            "no GetCapabilities effect should be emitted within the debounce window"
        );
    }

    #[test]
    fn refresh_capabilities_event_dispatches_after_debounce_elapses() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.authentication.last_capabilities_refresh_at =
            Some(Utc::now() - Duration::seconds(CAPABILITIES_REFRESH_DEBOUNCE_SECS + 1));

        let mut command = app.update(AppEvent::Authentication(AuthenticationEvent::RefreshCapabilities), &mut model, &());
        let _ = collect_effects(&mut command);

        assert!(model.authentication.is_refreshing_capabilities);
    }

    #[test]
    fn refresh_capabilities_event_skips_when_signed_out() {
        let app = BitBridge::default();
        let mut model = AppModel::default();

        let mut command = app.update(AppEvent::Authentication(AuthenticationEvent::RefreshCapabilities), &mut model, &());
        let effects = collect_effects(&mut command);

        assert!(!model.authentication.is_refreshing_capabilities);
        assert!(effects.is_empty());
    }

    #[test]
    fn capabilities_loaded_clears_in_flight_and_stamps_refresh_time() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.authentication.is_refreshing_capabilities = true;

        let _command = app.update(
            AppEvent::Authentication(AuthenticationEvent::CapabilitiesLoaded(paid_capabilities())),
            &mut model,
            &(),
        );

        assert!(!model.authentication.is_refreshing_capabilities);
        assert!(model.authentication.last_capabilities_refresh_at.is_some());
        assert_eq!(model.authentication.capabilities.as_ref().map(|c| c.plan), Some(Plan::Paid));
    }

    #[test]
    fn capabilities_loaded_idempotent_under_repeated_dispatch() {
        let app = BitBridge::default();
        let mut model = signed_in_model();

        let _ = app.update(
            AppEvent::Authentication(AuthenticationEvent::CapabilitiesLoaded(paid_capabilities())),
            &mut model,
            &(),
        );
        let plan_after_first = model.authentication.capabilities.as_ref().map(|c| c.plan);

        let _ = app.update(
            AppEvent::Authentication(AuthenticationEvent::CapabilitiesLoaded(paid_capabilities())),
            &mut model,
            &(),
        );
        let plan_after_second = model.authentication.capabilities.as_ref().map(|c| c.plan);

        assert_eq!(plan_after_first, plan_after_second);
    }

    #[test]
    fn refresh_capabilities_failed_clears_in_flight_flag() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.authentication.is_refreshing_capabilities = true;

        let _command = app.update(
            AppEvent::Authentication(AuthenticationEvent::RefreshCapabilitiesFailed("boom".to_owned())),
            &mut model,
            &(),
        );

        assert!(!model.authentication.is_refreshing_capabilities);
    }

    #[test]
    fn sign_out_clears_coordination_state() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.authentication.capabilities = Some(paid_capabilities());
        model.authentication.is_refreshing_capabilities = true;
        model.authentication.last_capabilities_refresh_at = Some(Utc::now());

        let _command = app.update(AppEvent::Authentication(AuthenticationEvent::SignOut), &mut model, &());

        assert!(model.authentication.user.is_none());
        assert!(model.authentication.capabilities.is_none());
        assert!(!model.authentication.is_refreshing_capabilities);
        assert!(model.authentication.last_capabilities_refresh_at.is_none());
    }

    #[test]
    fn shelf_view_model_can_create_shelf_reflects_capability_change() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        let mut free_with_one_shelf = UserCapabilities::free_defaults();
        free_with_one_shelf.presentation.max_visible_shelves = 1;
        model.authentication.capabilities = Some(free_with_one_shelf);
        model.shelf.shelves.push(crate::entities::shelf::Shelf::default());

        let view_free = app.view(&model);
        assert_eq!(view_free.shelf.as_ref().map(|s| s.can_create_shelf), Some(false));
        assert_eq!(view_free.shelf.as_ref().and_then(|s| s.max_shelves), Some(1));

        let _ = app.update(
            AppEvent::Authentication(AuthenticationEvent::CapabilitiesLoaded(paid_capabilities())),
            &mut model,
            &(),
        );

        let view_paid = app.view(&model);
        assert_eq!(view_paid.shelf.as_ref().map(|s| s.can_create_shelf), Some(true));
        assert_eq!(view_paid.shelf.as_ref().and_then(|s| s.max_shelves), None);
    }

    #[test]
    fn transfer_view_model_password_encryption_reflects_capability_change() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        model.authentication.capabilities = Some(UserCapabilities::free_defaults());

        let view_free = app.view(&model);
        assert_eq!(view_free.transfer.as_ref().map(|t| t.password_encryption_allowed), Some(false));

        let mut paid = paid_capabilities();
        paid.transfer_limits.password_encryption_allowed = true;
        let _ = app.update(
            AppEvent::Authentication(AuthenticationEvent::CapabilitiesLoaded(paid)),
            &mut model,
            &(),
        );

        let view_paid = app.view(&model);
        assert_eq!(view_paid.transfer.as_ref().map(|t| t.password_encryption_allowed), Some(true));
    }
}
