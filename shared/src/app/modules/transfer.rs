use crate::app::operations::database::{DatabaseOperation, TransferSessionDatabaseOperation};
use crate::app::operations::CoreOperation;
use crate::app::transfer::file_selection_service::ResourceSelection;
use crate::di_container::DiContainer;
use crate::entities::file::LocalResource;
use crate::{app::modules::AppModule, entities::transfer::TransferSession};
use crate::app::BitBridge;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    session: Option<TransferSession>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    session: Option<TransferSession>
}

#[derive(Default)]
pub struct TransferModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum TransferEvent {
    Init,
    UpdateSession(TransferSession),
    AddResource(LocalResource),
    SelectResource(ResourceSelection),
}

impl AppModule<BitBridge> for TransferModule {
    type Event = TransferEvent;
    type Model = TransferModel;
    type ViewModel = TransferViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            TransferEvent::Init => {
                Command::new(|it| async {
                    let transfer_service = DiContainer::get_instance().get_transfer_service();
                    transfer_service.update_current_transfer_session(it).await;
                })
            }
            TransferEvent::UpdateSession(session) => {
                model.session = Some(session);
                Command::new(|it| async {
                    TransferSessionDatabaseOperation::save_session(session).into_future(it).await;
                    it.request_from_shell(CoreOperation::Render).await;
                })
            }
            TransferEvent::SelectResource(resource) => {
                Command::new(|it| async {
                    let file_transfer_selection_service = DiContainer::get_instance().get_file_transfer_selection_service();
                    file_transfer_selection_service.add_resource(it, resource).await;
                })
            }
            TransferEvent::AddResource(resource) => {
                if let Some(session) = model.session.as_mut() {
                    session.add_resource(resource);

                    let saved_session = session.clone();
                    Command::new(|it| async {
                        TransferSessionDatabaseOperation::save_session(saved_session).into_future(it.clone()).await;
                        it.request_from_shell(CoreOperation::Render).await;
                    })
                }
                else {
                    Command::done()
                }
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        Self::ViewModel { session: model.session.clone() }
    }
}