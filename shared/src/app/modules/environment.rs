use core_services::logger;
use crux_core::{App, Command};
use schema::value::platform::Platform;
use serde::{Deserialize, Serialize};
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::BitBridge;
use crate::app::modules::AppModule;
use crate::di_container::DiContainer;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceInfo {
    pub platform: Platform,
    pub name: String,
    pub unique_id: String
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnvironmentModel {
    pub device: Option<DeviceInfo>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EnvironmentViewModel {}

pub struct EnvironmentModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum EnvironmentEvent {
    AppLaunched,
}

impl AppModule<BitBridge> for EnvironmentModule {
    type Model = EnvironmentModel;
    type ViewModel = EnvironmentViewModel;
    type Event = EnvironmentEvent;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            EnvironmentEvent::AppLaunched => {
                logger::setup();
                Command::new(|ctx| async {
                    let workdir_path = LocalStorageOperation::get_work_dir_path_cmd().into_future(ctx).await;
                    let di_container = DiContainer::get_instance();
                    di_container.init(workdir_path).await;
                })
                .then(Command::done())
            },
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        Self::ViewModel {}
    }
}
