use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::core::model_events::LocalResourceEvent;
use crate::app::modules::AppModule;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppModel, BitBridge};
use crate::entities::local_resource::{LocalResourcePath, ResourceType};
use crate::entities::shelf::Shelf;
use crate::repository::local_resource::LocalResourceId;
use core_services::db::repository::abstraction::table::Table;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ShelfModel {
    pub shelves: Vec<Shelf>,
    pub active_shelf_id: Option<u64>,
    pub is_loading: bool
}

impl ShelfModel {
    pub fn get_active_shelf(&self) -> Option<&Shelf> {
        match self.active_shelf_id {
            Some(id) => self.shelves.iter().find(|s| s.id == id),
            None => self.shelves.first()
        }
    }

    pub fn get_active_shelf_mut(&mut self) -> Option<&mut Shelf> {
        match self.active_shelf_id {
            Some(id) => self.shelves.iter_mut().find(|s| s.id == id),
            None => self.shelves.first_mut()
        }
    }

    pub fn get_shelf(&self, shelf_id: Option<u64>) -> Option<&Shelf> {
        match shelf_id {
            Some(id) => self.shelves.iter().find(|s| s.id == id),
            None => self.get_active_shelf()
        }
    }

    pub fn get_shelf_mut(&mut self, shelf_id: Option<u64>) -> Option<&mut Shelf> {
        match shelf_id {
            Some(id) => self.shelves.iter_mut().find(|s| s.id == id),
            None => self.get_active_shelf_mut()
        }
    }

    pub fn ensure_shelf_exists(&mut self) {
        if self.shelves.is_empty() {
            self.shelves.push(Shelf::new("Default"));
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShelfViewModel {
    pub selected_resources: Vec<SelectedResourceViewModel>,
    pub is_loading: bool
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShelfEvent {
    Launch,
    BeginLoadingResources,
    EndLoadingResources,
    OpenResource(u64),
    AddResources { shelf_id: Option<u64>, selections: Vec<ResourceSelection> },
    RemoveResource { shelf_id: Option<u64>, resource_id: u64 },
    Clear { shelf_id: Option<u64> },
    CreateShelf(String),
    DeleteShelf(u64),
    SwitchShelf(u64),

    #[serde(skip)]
    ShelfLoaded(Shelf),
    #[serde(skip)]
    ShelfCreated(Shelf),
    #[serde(skip)]
    ShelfDeleted(u64),
    #[serde(skip)]
    ModelEvent(LocalResourceEvent)
}

pub struct ShelfModule;

impl AppModule<BitBridge> for ShelfModule {
    type Event = ShelfEvent;
    type ViewModel = ShelfViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            Self::Event::Launch => {
                Command::handle_result(|it| async move { it.app().load_shelves().await })
            }
            Self::Event::BeginLoadingResources => {
                model.shelf.is_loading = true;
                Command::render()
            }
            Self::Event::EndLoadingResources => {
                model.shelf.is_loading = false;
                Command::render()
            }
            Self::Event::AddResources { shelf_id, selections } => {
                model.shelf.ensure_shelf_exists();
                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::done();
                };

                let mut commands = vec![];
                let mut filtered_selections = vec![];
                for selection in selections {
                    if shelf.is_exists(&selection.path) {
                        commands.push(Command::operate(DialogOperation::Toast(
                            "Resource was already added before.".to_owned()
                        )))
                    } else {
                        filtered_selections.push(selection);
                    }
                }

                let target_shelf_id = shelf.id;
                commands.push(Command::handle_result(move |it| async move {
                    let app = it.app();
                    app.notify_event(ShelfEvent::BeginLoadingResources);
                    app.new_resources(target_shelf_id, filtered_selections).await?;
                    app.notify_event(ShelfEvent::EndLoadingResources);

                    Ok(())
                }));

                Command::all(commands)
            }
            Self::Event::RemoveResource { shelf_id, resource_id } => {
                if model.transfer.has_active_send_session() {
                    return Command::operate(DialogOperation::Toast(
                        "Cannot remove resource during active transfer.".to_owned()
                    ));
                }

                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::done();
                };

                let Some(resource) = shelf.get(&LocalResourceId {
                    order_id: Some(resource_id),
                    ..Default::default()
                }) else {
                    return Command::done();
                };

                let id = resource.id();
                Command::handle_result(move |it| async move { it.app().remove_resource(id).await })
            }
            Self::Event::ModelEvent(event) => {
                match event {
                    LocalResourceEvent::Add { shelf_id, resource } => {
                        if let Some(shelf) = model.shelf.get_shelf_mut(shelf_id) {
                            shelf.add_resource(resource.clone());
                        }
                    }
                    LocalResourceEvent::Remove(id) => {
                        if let Some(shelf) = model.shelf.get_shelf_mut(id.shelf_id) {
                            shelf.remove_resource(&id);
                        }
                    }
                    _ => {}
                }

                Command::done()
            }
            Self::Event::OpenResource(id) => {
                let id = LocalResourceId {
                    order_id: Some(id),
                    ..Default::default()
                };

                let shelf = model.shelf.get_active_shelf();
                let Some(resource) = shelf.and_then(|s| s.get(&id)) else {
                    return Command::done();
                };

                let resource_path = resource.path.clone();
                Command::new(move |it| async move {
                    let _ = DeviceOperation::open(resource_path).into_future(it.clone()).await;
                })
            }
            Self::Event::Clear { shelf_id } => {
                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::done();
                };

                let commands = shelf
                    .resources
                    .iter()
                    .map(|it| {
                        let mut id = it.id();
                        id.shelf_id = shelf_id;
                        Command::handle_result(move |it| async move {
                            let _ = it.app().remove_resource(id).await;
                            Ok(())
                        })
                    })
                    .collect::<Vec<_>>();

                Command::all(commands)
            }
            Self::Event::CreateShelf(name) => {
                Command::handle_result(move |it| async move {
                    let shelf = it.app().create_shelf(name).await?;
                    it.app().notify_event(ShelfEvent::ShelfCreated(shelf));
                    Ok(())
                })
            }
            Self::Event::ShelfCreated(shelf) => {
                model.shelf.shelves.insert(0, shelf);
                Command::render()
            }
            Self::Event::DeleteShelf(shelf_id) => {
                if model.shelf.shelves.len() <= 1 {
                    return Command::operate(DialogOperation::Toast(
                        "Cannot delete the last shelf.".to_owned()
                    ));
                }

                Command::handle_result(move |it| async move {
                    it.app().delete_shelf(shelf_id).await?;
                    it.app().notify_event(ShelfEvent::ShelfDeleted(shelf_id));
                    Ok(())
                })
            }
            Self::Event::ShelfDeleted(shelf_id) => {
                model.shelf.shelves.retain(|s| s.id != shelf_id);
                if model.shelf.active_shelf_id == Some(shelf_id) {
                    model.shelf.active_shelf_id = None;
                }
                Command::render()
            }
            Self::Event::SwitchShelf(shelf_id) => {
                if model.shelf.shelves.iter().any(|s| s.id == shelf_id) {
                    model.shelf.active_shelf_id = Some(shelf_id);
                }
                Command::render()
            }
            Self::Event::ShelfLoaded(shelf) => {
                model.shelf.shelves.push(shelf);
                Command::done()
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        let resources = model
            .shelf
            .get_active_shelf()
            .map(|s| s.resources.iter().map(SelectedResourceViewModel::from).collect())
            .unwrap_or_default();

        ShelfViewModel {
            selected_resources: resources,
            is_loading: model.shelf.is_loading
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ResourceSelection {
    pub path: LocalResourcePath,
    // This is optional, if it is None, we will detect by Rust code to see if it should be a Folder or a File
    pub r#type: Option<ResourceType>
}
