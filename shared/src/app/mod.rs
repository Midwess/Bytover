use crux_core::{render::Render, App, Command};
use modules::AppModule;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use crate::app::modules::counter::{CounterEvent, CounterModel, CounterModule, CounterViewModel};

pub mod system;
pub mod modules;

#[derive(Default)]
pub struct BitBridge {
    pub counter: OnceCell<CounterModule>,
}

#[derive(Debug, Clone, Default)]
pub struct AppModel {
    counter: OnceCell<CounterModel>
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppViewModel {
    counter: Option<CounterViewModel>,
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
#[allow(unused)]
pub struct AppCapabilities {
    render: Render<AppEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    Counter(CounterEvent),
}

impl App for BitBridge {
    type Event = AppEvent;
    type Model = AppModel;
    type ViewModel = AppViewModel;
    type Capabilities = AppCapabilities;
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &Self::Capabilities,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            AppEvent::Counter(event) => {
                let counter = self.counter.get().unwrap();
                let model = model.counter.get_mut().unwrap();
                counter.update(event, model, caps)
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        println!("View model processing {:?}", model);
        AppViewModel {
            counter: Some(CounterViewModel {count: 2})
        }
    }
}
