use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

use crate::app::authentication::service::AuthenticationService;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::entities::user::User;

use super::nearby::NearbyEvent;
use super::AppModule;

pub struct AuthenticationModule {
    authentication_service: &'static AuthenticationService
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationModel {
    pub user: Option<User>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationViewModel {
    pub user: Option<User>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, uniffi::Enum)]
pub enum AuthenticationEvent {
    SignIn,
    SignUp,
    SignOut,
    OnRedirected { url: String },
    OnSignInSuccess { user: User }
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
            AuthenticationEvent::SignIn => Command::new(|ctx| async {
                self.authentication_service.signin(ctx).await;
            }),
            AuthenticationEvent::SignOut => Command::done(),
            AuthenticationEvent::OnRedirected { url } => Command::new(|ctx| async {
                self.authentication_service.handle_auth_response(url, ctx).await;
            }),
            AuthenticationEvent::SignUp => Command::done(),
            AuthenticationEvent::OnSignInSuccess { user } => {
                model.authentication.user.replace(user);
                Command::new(|ctx| async move {
                    ctx.send_event(AppEvent::Nearby(NearbyEvent::Launch()));
                })
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        AuthenticationViewModel {
            user: model.authentication.user.clone()
        }
    }
}
