use crate::app::file_system::workdir::WorkDir;
use crate::app::modules::AppModule;
use crate::app::operations::local_storage::LocalStorageOperation;
use crate::app::operations::CoreOperation;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::di_container::DiContainer;
use crate::entities::device::DeviceInfo;
use core_services::logger;
use crux_core::{App, Command};
use devlog_sdk::distributed_id::init_scoped_id_generator;
use serde::{Deserialize, Serialize};

use super::nearby::NearbyEvent;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnvironmentModel {
    pub device: Option<DeviceInfo>,
    pub workdir: Option<WorkDir>
}

impl EnvironmentModel {
    pub fn get_workdir(&self) -> &WorkDir {
        self.workdir.as_ref().unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EnvironmentViewModel {}

#[derive(Default)]
pub struct EnvironmentModule {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, uniffi::Enum)]
pub enum EnvironmentEvent {
    AppLaunched,
    UpdateWorkDir { workdir: WorkDir }
}

impl AppModule<BitBridge> for EnvironmentModule {
    type Event = EnvironmentEvent;
    type ViewModel = EnvironmentViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            EnvironmentEvent::AppLaunched => {
                logger::setup();
                init_scoped_id_generator("BitBridge".to_string());
                Command::new(|ctx| async move {
                    let workdir = LocalStorageOperation::get_work_dir_path_cmd().into_future(ctx.clone()).await;
                    let di_container = DiContainer::get_instance();
                    di_container.init(workdir.clone());
                    ctx.request_from_shell(CoreOperation::InitNativeExecutor).await;
                    // di_container.get_authentication_service().update_signin_session(ctx).await;
                    log::info!(target: "nearby", "Starting");
                    ctx.notify_shell(CoreOperation::Notified(AppEvent::Environment(
                        EnvironmentEvent::UpdateWorkDir { workdir }
                    )));
                    ctx.request_from_shell(CoreOperation::Notified(AppEvent::Nearby(NearbyEvent::Launch()))).await;
                })
                .then(Command::done())
            }
            EnvironmentEvent::UpdateWorkDir { workdir } => {
                model.environment.workdir = Some(workdir);
                Command::done()
            }
        }
    }

    fn view(&self, _model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {}
    }
}
