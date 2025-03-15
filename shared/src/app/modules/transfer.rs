use crate::app::operations::transfer::TransferOperation;
use crate::app::operations::CoreOperation;
use crate::app::transfer::file_selection_service::ResourceSelection;
use crate::app::transfer::transfer_selection::TransferMethodSelection;
use crate::app::{modules::AppModule, view_models::selected_resource::SelectedResourceViewModel};
use crate::app::BitBridge;
use crate::di_container::DiContainer;
use crate::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferModel {
    selected_resources: Vec<LocalResource>,
    transfer_method_selection: TransferMethodSelection,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    selected_resources: Vec<SelectedResourceViewModel>,
    transfer_method_selection: TransferMethodSelection
}

#[derive(Default)]
pub struct TransferModule {}

#[derive(Clone, Debug, Serialize, Deserialize, uniffi::Enum)]
pub enum TransferEvent {
    // Event from shell 
    Launched(),
    AddResources(Vec<ResourceSelection>),
    RemoveResource(u64),

    // Event from core
    UpdateResourcesModel {
        new: Vec<LocalResource>,
        removed: Vec<LocalResource>
    }
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
            TransferEvent::Launched() => {
                Command::new(|it| async move {
                    let resource_transfer_selection_service =
                        DiContainer::get_instance().get_resource_transfer_selection_service();
                    resource_transfer_selection_service.load_resources(it.clone()).await;
                    let nearby_service = DiContainer::get_instance().get_nearby_service();
                    nearby_service.init(it.clone()).await;
                })
            }
            TransferEvent::AddResources(selections) => {
                Command::new(|it| async move {
                    let resource_transfer_selection_service =
                        DiContainer::get_instance().get_resource_transfer_selection_service();
                    resource_transfer_selection_service.add_resources(it, selections).await;
                })
            }
            TransferEvent::RemoveResource(id) => {
                Command::new(|it| async move {
                    let resource_transfer_selection_service =
                        DiContainer::get_instance().get_resource_transfer_selection_service();
                    resource_transfer_selection_service.remove_resource(it, id).await;
                })
            }
            TransferEvent::UpdateResourcesModel { new, removed } => {
                model.selected_resources.extend(new);
                model.selected_resources.retain(|it| !removed.iter().any(|removed| removed.order_id == it.order_id));

                Command::done()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        Self::ViewModel {
            selected_resources: model.selected_resources.iter().map(|it| SelectedResourceViewModel::from(it)).collect(),
            transfer_method_selection: model.transfer_method_selection.clone()
        }
    }
}
