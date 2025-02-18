pub mod system;
pub mod modules;
pub mod operations;
pub mod authentication;

// pub mod bridge;
use std::{future::Future, process::Output};

use crux_core::{capability::CapabilityContext, command::{CommandContext, RequestBuilder}, macros::Capability, render::Render, App, Command};
use modules::{authentication::{AuthenticationEvent, AuthenticationModel, AuthenticationModule, AuthenticationViewModel}, AppModule};
use operations::CoreOperation;
use serde::{Deserialize, Serialize};
use crate::{app::modules::environment::{EnvironmentEvent, EnvironmentModel, EnvironmentModule, EnvironmentViewModel}, di_container::DiContainer};

pub type AppCommand = Command<<BitBridge as App>::Effect, <BitBridge as App>::Event>;
pub type AppCommandContext = CommandContext<<BitBridge as App>::Effect, <BitBridge as App>::Event>;
pub type AppRequestBuilder<T: Future<Output = T>> = RequestBuilder<<BitBridge as App>::Effect, <BitBridge as App>::Event, T>;

#[derive(Default)]
pub struct BitBridge {
    environment: EnvironmentModule,
    authentication: AuthenticationModule
}

#[derive(Debug, Clone, Default)]
pub struct AppModel {
    environment: EnvironmentModel,
    authentication: AuthenticationModel
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppViewModel {
    environment: Option<EnvironmentViewModel>,
    authentication: Option<AuthenticationViewModel>
}

// The capability in CRUX has been deprecated by command API
// instead it just be here to be used for generating effect
#[derive(Capability, Clone)]
pub struct AppCapabilities<Ev> {
    context: CapabilityContext<CoreOperation, Ev>,
}

impl<Ev> AppCapabilities<Ev> where Ev: 'static {
    pub fn new(context: CapabilityContext<CoreOperation, Ev>) -> Self {
        Self { context }
    }
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
#[allow(unused)]
pub struct AppEffect {
    capabilities: AppCapabilities<AppEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    Environment(EnvironmentEvent),
    Authentication(AuthenticationEvent),
    Void
}

pub type BitBridgeEffect = Effect;

impl App for BitBridge {
    type Event = AppEvent;
    type Model = AppModel;
    type ViewModel = AppViewModel;
    type Capabilities = AppEffect;
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &Self::Capabilities,
    ) -> Command<Self::Effect, Self::Event> {
        log::info!(target: "app-update", "Updating app with event {:?}", event);
        match event {
            AppEvent::Environment(event) => {
                let model = &mut model.environment;
                self.environment.update(event, model, caps)
            },
            AppEvent::Authentication(event) => {
                let model = &mut model.authentication;
                self.authentication.update(event, model, caps)
            },
            AppEvent::Void => {
                Command::done()
            },
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        AppViewModel {
            environment: Some(self.environment.view(&model.environment)),
            authentication: Some(self.authentication.view(&model.authentication))
        }
    }
}
