use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::modules::AppModule;
use crate::app::operations::CoreOperation;
use crate::app::{AppModel, BitBridge};
use crate::entities::device::DeviceInfo;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnvironmentModel {
    pub device: Option<DeviceInfo>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EnvironmentViewModel {}

pub struct EnvironmentModule;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EnvironmentEvent {
    AppLaunched
}

impl AppModule<BitBridge> for EnvironmentModule {
    type Event = EnvironmentEvent;
    type ViewModel = EnvironmentViewModel;

    fn update(
        &self,
        event: Self::Event,
        _model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            EnvironmentEvent::AppLaunched => Command::new_result(|ctx| async move {
                ctx.request_from_shell(CoreOperation::InitNativeExecutor).await;
                ctx.app().re_authorize().await?;

                Ok(())
            }).then_render()
        }
    }

    fn view(&self, _model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {}
    }
}
