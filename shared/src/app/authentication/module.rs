use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::{AppModel, BitBridge};
use crate::entities::capabilities::UserCapabilities;
use crate::entities::user::User;
use core_services::utils::string::StringExt;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

use crate::app::modules::AppModule;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::rpc::RpcOperation;
use crate::app::AppEvent;
use crate::CoreOperation;

pub struct AuthenticationModule;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationModel {
    pub user: Option<User>,
    pub capabilities: Option<UserCapabilities>,
    pub is_already_feedback: bool,
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
                let is_authorized = model.authentication.user.is_some();

                model.authentication.user.replace(user);
                if is_authorized {
                    return Command::done();
                }

                let fetch_caps = Command::new(|it| async move {
                    match RpcOperation::get_capabilities().into_future(it.clone()).await {
                        Ok(caps) => {
                            it.app().notify_event(AppEvent::Authentication(AuthenticationEvent::CapabilitiesLoaded(caps)));
                        }
                        Err(err) => {
                            log::warn!("Failed to load user capabilities: {:?}", err);
                        }
                    }
                });

                Command::all(vec![Command::render(), fetch_caps])
            }
            AuthenticationEvent::CapabilitiesLoaded(caps) => {
                let limit = caps.shelf_limit();
                model.authentication.capabilities.replace(caps);

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
