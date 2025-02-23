use crate::app::modules::AppModule;
use crate::app::operations::database::TransferSessionDatabaseOperation;
use crate::app::operations::CoreOperation;
use crate::app::transfer::file_selection_service::ResourceSelection;
use crate::app::BitBridge;
use crate::di_container::DiContainer;
use crate::entities::file::LocalResource;
use crate::entities::transfer::TransferSession;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    session: Option<TransferSession>
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    session: Option<TransferSession>,
    selected_resources: Vec<LocalResource>
}

#[derive(Default)]
pub struct TransferModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum TransferEvent {
    InitSession,
    UpdateSession(TransferSession),
    UpdateLocalResources(Vec<LocalResource>),
    AddResourceSelections(Vec<ResourceSelection>)
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
            TransferEvent::InitSession => Command::new(|it| async {
                let transfer_service = DiContainer::get_instance().get_transfer_service();
                transfer_service.update_current_transfer_session(it).await;
            }),
            TransferEvent::UpdateSession(session) => {
                model.session = Some(session);
                let session = model.session.clone().unwrap();
                Command::new(|it| async move {
                    TransferSessionDatabaseOperation::save_session(session).into_future(it.clone()).await;
                    it.request_from_shell(CoreOperation::Render).await;
                })
            }
            TransferEvent::AddResourceSelections(selections) => {
                let selection_from_core =
                    model.session.as_ref().expect("Session must be initialized").resources.clone();
                Command::new(|it| async move {
                    let resource_transfer_selection_service =
                        DiContainer::get_instance().get_resource_transfer_selection_service();
                    resource_transfer_selection_service.add_resources(it, selection_from_core, selections).await;
                })
            }
            TransferEvent::UpdateLocalResources(resources) => {
                if let Some(session) = model.session.as_mut() {
                    session.resources = resources;

                    let saved_session = session.clone();
                    Command::new(|it| async move {
                        TransferSessionDatabaseOperation::save_session(saved_session).into_future(it.clone()).await;
                        it.request_from_shell(CoreOperation::Render).await;
                    })
                } else {
                    Command::done()
                }
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        Self::ViewModel {
            session: model.session.clone(),
            selected_resources: match model.session.as_ref() {
                Some(session) => session.resources.clone(),
                None => vec![]
            }
        }
    }
}
