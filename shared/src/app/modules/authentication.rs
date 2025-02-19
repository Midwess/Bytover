use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

use crate::app::BitBridge;
use crate::di_container::DiContainer;
use crate::entities::user::User;

use super::AppModule;

#[derive(Default)]
pub struct AuthenticationModule {}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationModel {
    pub user: Option<User>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationViewModel {
    pub user: Option<User>
}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum AuthenticationEvent {
    SignIn,
    SignUp,
    SignOut,
    OnRedirected { url: String },
    OnSignInSuccess { user: User }
}

impl AppModule<BitBridge> for AuthenticationModule {
    type Event = AuthenticationEvent;
    type Model = AuthenticationModel;
    type ViewModel = AuthenticationViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            AuthenticationEvent::SignIn => Command::new(|ctx| async {
                let auth_service = DiContainer::get_instance().get_authentication_service();
                auth_service.signin(ctx).await;
            }),
            AuthenticationEvent::SignOut => Command::done(),
            AuthenticationEvent::OnRedirected { url } => Command::new(|ctx| async {
                let auth_service = DiContainer::get_instance().get_authentication_service();
                auth_service.handle_auth_response(url, ctx).await;
            }),
            AuthenticationEvent::SignUp => Command::done(),
            AuthenticationEvent::OnSignInSuccess { user } => {
                model.user.replace(user);
                Command::done()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        AuthenticationViewModel {
            user: model.user.clone()
        }
    }
}
