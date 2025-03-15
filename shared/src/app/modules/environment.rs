use crate::app::modules::AppModule;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::BitBridge;
use crate::di_container::DiContainer;
use core_services::logger;
use crux_core::{App, Command};
use devlog_sdk::distributed_id::init_scoped_id_generator;
use schema::value::platform::Platform;
use serde::{Deserialize, Serialize};
use uniffi::Record;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Record)]
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

#[derive(Default)]
pub struct EnvironmentModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum EnvironmentEvent {
    AppLaunched
}

impl AppModule<BitBridge> for EnvironmentModule {
    type Event = EnvironmentEvent;
    type Model = EnvironmentModel;
    type ViewModel = EnvironmentViewModel;

    fn update(
        &self,
        event: Self::Event,
        _model: &mut Self::Model,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            EnvironmentEvent::AppLaunched => {
                logger::setup();
                init_scoped_id_generator("BitBridge".to_string());
                Command::new(|ctx| async move {
                    let workdir_path = LocalStorageOperation::get_work_dir_path_cmd().into_future(ctx.clone()).await;
                    let di_container = DiContainer::get_instance();
                    di_container.init(workdir_path).await;
                    // di_container.get_authentication_service().update_signin_session(ctx).await;
                })
                .then(Command::done())
            }
        }
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        Self::ViewModel {}
    }
}
