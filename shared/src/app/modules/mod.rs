pub mod authentication;
pub mod environment;
pub mod transfer;

use crux_core::{App, Command};

use super::AppModel;

pub trait AppModule<T>
where
    T: App
{
    type ViewModel;
    type Event;

    fn update(&self, event: Self::Event, model: &mut AppModel, caps: &T::Capabilities) -> Command<T::Effect, T::Event>;

    fn view(&self, model: &AppModel) -> Self::ViewModel;
}
