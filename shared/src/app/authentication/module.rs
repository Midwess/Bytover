use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::payment::module::PaymentEvent;
use crate::app::{AppModel, BitBridge};
use crate::entities::user::User;
use core_services::utils::string::StringExt;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

use crate::app::modules::AppModule;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::AppEvent;
use crate::CoreOperation;

pub struct AuthenticationModule;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationModel {
    pub user: Option<User>,
    pub is_already_feedback: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationViewModel {
    pub user: Option<User>,
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
                let do_sign_out = Command::handle_result(|ctx| async move {
                    ctx.app().sign_out().await?;
                    Ok(())
                });
                Command::all(vec![Command::render(), do_sign_out])
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
                    it.app().notify_event(AppEvent::Payment(PaymentEvent::RefreshCapabilities));
                });

                Command::all(vec![Command::render(), request_refresh])
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
            is_already_feedback: model.authentication.is_already_feedback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppEvent, BitBridge};
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

    #[test]
    fn sign_out_clears_user() {
        let app = BitBridge::default();
        let mut model = signed_in_model();

        let _command = app.update(AppEvent::Authentication(AuthenticationEvent::SignOut), &mut model, &());

        assert!(model.authentication.user.is_none());
    }

    #[test]
    fn shelf_view_model_can_create_shelf_reflects_capability_change() {
        let app = BitBridge::default();
        let mut model = signed_in_model();
        let mut free_with_one_shelf = UserCapabilities::free_defaults();
        free_with_one_shelf.presentation.max_visible_shelves = 1;
        model.payment.capabilities = Some(free_with_one_shelf);
        model.shelf.shelves.push(crate::entities::shelf::Shelf::default());

        let view_free = app.view(&model);
        assert_eq!(view_free.shelf.as_ref().map(|s| s.can_create_shelf), Some(false));
        assert_eq!(view_free.shelf.as_ref().and_then(|s| s.max_shelves), Some(1));

        let _ = app.update(
            AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(paid_capabilities())),
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
        model.payment.capabilities = Some(UserCapabilities::free_defaults());

        let view_free = app.view(&model);
        assert_eq!(view_free.transfer.as_ref().map(|t| t.password_encryption_allowed), Some(false));

        let mut paid = paid_capabilities();
        paid.transfer_limits.password_encryption_allowed = true;
        let _ = app.update(
            AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(paid)),
            &mut model,
            &(),
        );

        let view_paid = app.view(&model);
        assert_eq!(view_paid.transfer.as_ref().map(|t| t.password_encryption_allowed), Some(true));
    }
}
