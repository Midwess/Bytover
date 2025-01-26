use crux_core::{
    render::{render, Render},
    App, Command,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CounterEvent {
    Increment,
    Decrement,
    Reset,
}

#[derive(Default)]
pub struct CounterModel {
    count: isize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CounterViewModel {
    pub count: String,
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
#[allow(unused)]
pub struct CounterCapabilities {
    render: Render<CounterEvent>,
}

#[derive(Default)]
pub struct Counter;

impl App for Counter {
    type Event = CounterEvent;
    type Model = CounterModel;
    type ViewModel = CounterViewModel;
    type Capabilities = CounterCapabilities;
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &Self::Capabilities,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            CounterEvent::Increment => {
                log::info!(target: "tiendang-debug", "Incrementing");
                model.count += 1;
            }
            CounterEvent::Decrement => {
                log::info!(target: "tiendang-debug", "Decrementing");
                model.count -= 1;
            }
            CounterEvent::Reset => {
                model.count = 0;
            }
        };

        render()
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        CounterViewModel {
            count: format!("Count is: {}", model.count),
        }
    }
}
