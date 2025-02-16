use crux_core::{App, Command};
use schema::devlog::auth_gateway::rpc::{auth_service_client::AuthServiceClient, SigninRequest};
use serde::{Deserialize, Serialize};

use crate::app::{authentication::service::AuthenticationService, operations::{device::DeviceOperation, webview::WebViewOperation, CoreOperation}, BitBridge};

use super::AppModule;

pub struct AuthenticationModule {
    pub auth_service: &'static AuthenticationService
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationModel {}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthenticationViewModel {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum AuthenticationEvent {
    SignIn,
    SignUp,
    SignOut,
    OnRedirected { url: String }
}

impl AppModule<BitBridge> for AuthenticationModule {
    type Model = AuthenticationModel;
    type ViewModel = AuthenticationViewModel;
    type Event = AuthenticationEvent;

    fn update(
        &self, 
        event: Self::Event, 
        model: &mut Self::Model,
        caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        let cap_ctx = caps.capabilities.context.clone();
        match event {
            AuthenticationEvent::SignIn => {
                Command::new(|ctx| async {
                    self.auth_service.signin(ctx).await;
                })
            }
            AuthenticationEvent::SignOut => {
                Command::done()
            }
            AuthenticationEvent::OnRedirected { url } => {
                Command::new(|ctx| async {
                    self.auth_service.handle_auth_response(url).await;
                })
                .then(Command::done())
            },
            AuthenticationEvent::SignUp => {
                Command::done()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        AuthenticationViewModel {}
    }
}
