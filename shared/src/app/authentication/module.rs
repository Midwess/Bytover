use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

use crate::app::core_utils::CoreCommandContextUtils;
use crate::app::modules::transfer::TransferEvent;
use crate::app::{AppModel, BitBridge};
use crate::entities::user::User;

use crate::app::modules::AppModule;
use crate::app::nearby::module::NearbyEvent;
use crate::app::shelf::module::ShelfEvent;

pub struct AuthenticationModule;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationModel {
    pub user: Option<User>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationViewModel {
    pub user: Option<User>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthenticationEvent {
    SignIn,
    SignUp,
    SignOut,
    OnRedirected { url: String },
    UpdateUser { user: User }
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
            AuthenticationEvent::SignIn => Command::new(|ctx| async move {
                ctx.app().sign_in().await;
            }),
            AuthenticationEvent::SignOut => Command::done(),
            AuthenticationEvent::OnRedirected { url } => Command::new(|ctx| async move {
                ctx.app().authorize(url).await;
            }),
            AuthenticationEvent::SignUp => Command::done(),
            AuthenticationEvent::UpdateUser { user } => {
                model.authentication.user.replace(user);
                Command::new(|ctx| async move {
                    ctx.notify_event(ShelfEvent::Launch);
                    ctx.notify_event(TransferEvent::Launch);
                    ctx.notify_event(NearbyEvent::Launch);
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
