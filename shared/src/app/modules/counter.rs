use crux_core::{App, Command};
use serde::{Deserialize, Serialize};
use crate::app::BitBridge;
use crate::app::modules::AppModule;

#[derive(Clone, Debug, Default)]
pub struct CounterModel {}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CounterViewModel {
    pub count: i32
}

#[derive(Default)]
pub struct CounterModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum CounterEvent {
    Increment,
    Decrement
}

impl AppModule<BitBridge> for CounterModule {
    type Model = CounterModel;
    type ViewModel = CounterViewModel;
    type Event = CounterEvent;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        todo!()
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        todo!()
    }
}
