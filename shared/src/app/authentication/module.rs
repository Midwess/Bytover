use crux_core::{App, Command};
use serde::{Deserialize, Serialize};
use core_services::utils::string::StringExt;
use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::{AppModel, BitBridge};
use crate::entities::user::User;

use crate::app::modules::AppModule;
use crate::app::nearby::module::NearbyEvent;
use crate::app::operations::dialog::DialogOperation;
use crate::app::operations::p2p::P2POperation;
use crate::app::operations::rpc::RpcOperation;
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
    // User not signed in
    UnAuthorized,
    Feedback { message: Option<String>, email: String }
}

impl AppModule<BitBridge> for AuthenticationModule {
    type Event = AuthenticationEvent;
    type ViewModel = AuthenticationViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            AuthenticationEvent::Authenticate => Command::handle_result(|ctx| async move {
                ctx.app().authenticate().await;
                Ok(())
            }),
            AuthenticationEvent::SignOut => {
                model.authentication.user.take();
                if !model.environment.auto_launch_nearby || !model.environment.allowed_nearby_anonymous {
                    return Command::new(|it| async move {
                        it.app().run(P2POperation::stop()).await;
                    }).then_render();
                }

                Command::handle_result(|ctx| async move {
                    ctx.app().sign_out().await?;
                    ctx.notify_shell(CoreOperation::Render);
                    let _ = ctx.app().restart_nearby(true).await;
                    Ok(())
                })
            },
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

                if !model.environment.auto_launch_nearby {
                    return Command::render()
                }

                Command::new(|ctx| async move {
                    let app = ctx.app();
                    let _ = app.restart_nearby(true).await;
                })
            }
            AuthenticationEvent::UnAuthorized => {
                if !model.environment.auto_launch_nearby || !model.environment.allowed_nearby_anonymous {
                    return Command::render();
                }

                Command::new(|ctx| async move {
                    let app = ctx.app();
                    app.notify_event(NearbyEvent::Launch { auto_launch: true });
                })
            }
            AuthenticationEvent::Feedback {email, message} => {
                if let Some(message) = message.as_ref() {
                    if message.len() > 4024 {
                        return Command::new(|it| async move {
                            it.app().run(DialogOperation::toast("Message is too long".to_string())).await;
                        })
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
            is_already_feedback: model.authentication.is_already_feedback
        }
    }
}
