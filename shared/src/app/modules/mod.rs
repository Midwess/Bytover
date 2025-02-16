pub mod environment;
pub mod authentication;

use crux_core::{App, Command};

pub trait AppModule<T> where T: App {
    type Model;
    type ViewModel;
    type Event;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &T::Capabilities,
    ) -> Command<T::Effect, T::Event>;

    fn view(&self, model: &Self::Model) -> Self::ViewModel;
}
