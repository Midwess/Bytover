pub mod authentication;
pub mod core_utils;
pub mod file_system;
pub mod modules;
pub mod nearby;
pub mod operations;
pub mod repository;
pub mod system;
pub mod transfer;
pub mod view_models;

// pub mod bridge;

use crate::app::modules::environment::{EnvironmentEvent, EnvironmentModel, EnvironmentModule, EnvironmentViewModel};
use crux_core::capability::CapabilityContext;
use crux_core::command::{CommandContext, RequestBuilder};
use crux_core::macros::Capability;
use crux_core::{App, Command};
use modules::authentication::{AuthenticationEvent, AuthenticationModel, AuthenticationModule, AuthenticationViewModel};
use modules::nearby::{NearbyEvent, NearbyModel, NearbyModule, NearbyViewModel};
use modules::transfer::{TransferEvent, TransferModel, TransferModule, TransferViewModel};
use modules::AppModule;
use operations::CoreOperation;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

pub type AppCommand = Command<<BitBridge as App>::Effect, <BitBridge as App>::Event>;
pub type AppCommandContext = CommandContext<<BitBridge as App>::Effect, <BitBridge as App>::Event>;
pub type AppRequestBuilder<T> = RequestBuilder<<BitBridge as App>::Effect, <BitBridge as App>::Event, T>;

#[derive(Default)]
pub struct BitBridge {
    environment: OnceCell<EnvironmentModule>,
    authentication: OnceCell<AuthenticationModule>,
    transfer: OnceCell<TransferModule>,
    nearby: OnceCell<NearbyModule>
}

#[derive(Debug, Clone, Default)]
pub struct AppModel {
    environment: EnvironmentModel,
    authentication: AuthenticationModel,
    transfer: TransferModel,
    nearby: NearbyModel
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppViewModel {
    environment: Option<EnvironmentViewModel>,
    authentication: Option<AuthenticationViewModel>,
    transfer: Option<TransferViewModel>,
    nearby: Option<NearbyViewModel>
}

// The capability in CRUX has been deprecated by command API
// instead it just be here to be used for generating effect
#[derive(Capability, Clone)]
pub struct AppCapabilities<Ev> {
    context: CapabilityContext<CoreOperation, Ev>
}

impl<Ev> AppCapabilities<Ev>
where
    Ev: 'static
{
    pub fn new(context: CapabilityContext<CoreOperation, Ev>) -> Self {
        Self { context }
    }
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
#[allow(unused)]
pub struct AppEffect {
    capabilities: AppCapabilities<AppEvent>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppEvent {
    Environment(EnvironmentEvent),
    Authentication(AuthenticationEvent),
    Transfer(TransferEvent),
    Nearby(NearbyEvent),
    Void
}

pub type BitBridgeEffect = Effect;

impl App for BitBridge {
    type Capabilities = AppEffect;
    type Effect = Effect;
    type Event = AppEvent;
    type Model = AppModel;
    type ViewModel = AppViewModel;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) -> Command<Self::Effect, Self::Event> {
        match event {
            AppEvent::Environment(event) => self.environment.get().unwrap().update(event, model, caps),
            AppEvent::Authentication(event) => self.authentication.get().unwrap().update(event, model, caps),
            AppEvent::Transfer(event) => self.transfer.get().unwrap().update(event, model, caps),
            AppEvent::Nearby(event) => self.nearby.get().unwrap().update(event, model, caps),
            AppEvent::Void => Command::done()
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        AppViewModel {
            environment: Some(self.environment.get().unwrap().view(model)),
            authentication: Some(self.authentication.get().unwrap().view(model)),
            transfer: Some(self.transfer.get().unwrap().view(model)),
            nearby: Some(self.nearby.get().unwrap().view(model))
        }
    }
}
