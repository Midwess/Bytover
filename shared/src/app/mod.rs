use std::{cell::OnceCell, collections};

use crate::app::counter::Counter;
use counter::CounterEvent;
use crux_core::{render::Render, App, Command, Core};
use serde::{Deserialize, Serialize};

pub mod system;
pub mod counter;

#[derive(Default)]
pub struct BitBridge {
    pub counter: OnceCell<Core<Counter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppModel {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppViewModel {}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
#[allow(unused)]
pub struct AppCapabilities {
    render: Render<AppEvent>,
}

pub enum AppEffect {
    Counter(Box<counter::Effect>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    Counter(Box<CounterEvent>),
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
                let counter = self.counter.get_or_init(|| Core::new());
                Command::done()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        AppViewModel {}
    }
}