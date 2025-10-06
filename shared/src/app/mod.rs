pub mod authentication;
pub mod core;
pub mod core_utils;
pub mod environment;
pub mod modules;
pub mod nearby;
pub mod operations;
pub mod shelf;
pub mod transfer;
pub mod view_models;

pub use crate::app::operations::CoreOperation;

use crate::app::shelf::module::{ShelfEvent, ShelfModel, ShelfModule, ShelfViewModel};
use crate::app::transfer::transfer_service::TransferService;
use authentication::module::{AuthenticationEvent, AuthenticationModel, AuthenticationModule, AuthenticationViewModel};
use crux_core::command::{CommandContext, RequestBuilder};
use crux_core::macros::effect;
use crux_core::{App, Command};
use derive_more::From;
use environment::module::{EnvironmentEvent, EnvironmentModule, EnvironmentViewModel};
use modules::transfer::{TransferEvent, TransferModel, TransferModule, TransferViewModel};
use modules::AppModule;
use nearby::module::{NearbyEvent, NearbyModel, NearbyModule, NearbyViewModel};
use serde::{Deserialize, Serialize};

pub type AppCommand = Command<<BitBridge as App>::Effect, <BitBridge as App>::Event>;
pub type AppCommandContext = CommandContext<<BitBridge as App>::Effect, <BitBridge as App>::Event>;
pub type AppRequestBuilder<T> = RequestBuilder<<BitBridge as App>::Effect, <BitBridge as App>::Event, T>;

pub struct BitBridge {
    environment: EnvironmentModule,
    authentication: AuthenticationModule,
    transfer: TransferModule,
    nearby: NearbyModule,
    shelf: ShelfModule
}

impl Default for BitBridge {
    fn default() -> Self {
        Self {
            environment: EnvironmentModule,
            authentication: AuthenticationModule,
            shelf: ShelfModule,
            transfer: TransferModule {
                transfer_service: TransferService::instance()
            },
            nearby: NearbyModule
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppModel {
    authentication: AuthenticationModel,
    transfer: TransferModel,
    nearby: NearbyModel,
    shelf: ShelfModel
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppViewModel {
    environment: Option<EnvironmentViewModel>,
    authentication: Option<AuthenticationViewModel>,
    transfer: Option<TransferViewModel>,
    nearby: Option<NearbyViewModel>,
    shelf: Option<ShelfViewModel>
}

/// The effects that shell need to handle
/// - This is not exactly best practice of crux_core, because I didn't see it best fit for this project
#[effect(typegen)]
#[derive(Debug)]
pub enum AppOperation {
    Operation(CoreOperation)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From)]
pub enum AppEvent {
    Environment(EnvironmentEvent),
    Authentication(AuthenticationEvent),
    Transfer(TransferEvent),
    Nearby(NearbyEvent),
    Shelf(ShelfEvent),
    Void
}

impl App for BitBridge {
    type Capabilities = ();
    type Effect = AppOperation;
    type Event = AppEvent;
    type Model = AppModel;
    type ViewModel = AppViewModel;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) -> Command<Self::Effect, Self::Event> {
        match event {
            AppEvent::Environment(event) => self.environment.update(event, model, caps),
            AppEvent::Authentication(event) => self.authentication.update(event, model, caps),
            AppEvent::Transfer(event) => self.transfer.update(event, model, caps),
            AppEvent::Nearby(event) => self.nearby.update(event, model, caps),
            AppEvent::Shelf(event) => self.shelf.update(event, model, caps),
            AppEvent::Void => Command::done()
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        AppViewModel {
            environment: Some(self.environment.view(model)),
            authentication: Some(self.authentication.view(model)),
            transfer: Some(self.transfer.view(model)),
            nearby: Some(self.nearby.view(model)),
            shelf: Some(self.shelf.view(model))
        }
    }
}
