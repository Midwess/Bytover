use crate::app::modules::AppModule;
use crate::app::operations::CoreOperation;
use crate::app::{AppEvent, AppModel, BitBridge};
use crate::entities::device::DeviceInfo;
use core_services::logger;
use crux_core::{App, Command};
use devlog_sdk::distributed_id::init_scoped_id_generator;
use serde::{Deserialize, Serialize};
use uniffi::Enum;
use crate::app::authentication::service::AuthenticationService;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnvironmentModel {
    pub device: Option<DeviceInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EnvironmentViewModel {}

pub struct EnvironmentModule {
    authentication_service: &'static AuthenticationService
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Enum)]
pub enum EnvironmentEvent {
    AppLaunched,
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
                let authentication_service = self.authentication_service;
                Command::new(|ctx| async move {
                    ctx.request_from_shell(CoreOperation::InitNativeExecutor).await;
                    authentication_service.update_signin_session(ctx.clone()).await;
                })
                .then(Command::done())
            }
        }
    }

    fn view(&self, _model: &AppModel) -> Self::ViewModel {
        Self::ViewModel {}
    }
}
