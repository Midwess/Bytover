use crate::app::modules::AppModule;
use crate::app::BitBridge;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {}

#[derive(Default)]
pub struct TransferModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum TransferEvent {}

impl AppModule<BitBridge> for TransferModule {
    type Event = TransferEvent;
    type Model = TransferModel;
    type ViewModel = TransferViewModel;

    fn update(
        &self,
        event: Self::Event,
        _model: &mut Self::Model,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {}
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        Self::ViewModel {}
    }
}
