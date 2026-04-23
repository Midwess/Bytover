use crate::app::core::extensions::{CoreCommandContextUtils, CoreCommandUtils};
use crate::app::core::model_events::{LocalResourceEvent, LocalResourceUpdateEvent};
use crate::app::modules::AppModule;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::dialog::DialogOperation;
use crate::app::view_models::peer_avatar::PeerAvatarViewModel;
use crate::app::view_models::selected_resource::SelectedResourceViewModel;
use crate::app::{AppModel, BitBridge};
use crate::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use crate::entities::shelf::Shelf;
use crate::repository::local_resource::LocalResourceId;
use core_services::db::repository::abstraction::table::Table;
use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ShelfModel {
    pub shelves: Vec<Shelf>,
    pub is_loading: bool,
}

impl ShelfModel {
    pub fn get_shelf(&self, shelf_id: u64) -> Option<&Shelf> {
        self.shelves.iter().find(|s| s.id == shelf_id)
    }

    pub fn get_shelf_mut(&mut self, shelf_id: u64) -> Option<&mut Shelf> {
        self.shelves.iter_mut().find(|s| s.id == shelf_id)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShelfItemViewModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_online: bool,
    pub is_resource_remove_allowed: bool,
    pub resources: Vec<SelectedResourceViewModel>,
}

impl ShelfItemViewModel {
    pub fn from_shelf(shelf: &Shelf, is_online: bool) -> Self {
        use chrono::Local;
        let created_date = shelf.created_at().with_timezone(&Local);
        let description = created_date.format("%b %d, %Y %I:%M %p").to_string();

        Self {
            id: shelf.id.to_string(),
            name: shelf.name.clone(),
            description,
            is_online,
            is_resource_remove_allowed: true,
            resources: shelf.resources.iter().map(SelectedResourceViewModel::from).collect(),
        }
    }
}

impl From<&Shelf> for ShelfItemViewModel {
    fn from(shelf: &Shelf) -> Self {
        Self::from_shelf(shelf, false)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShelfViewModel {
    pub shelves: Vec<ShelfItemViewModel>,
    pub is_loading: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShelfEvent {
    Launch,
    BeginLoadingResources,
    EndLoadingResources,
    ValidateLoadedResources(Vec<LocalResource>),
    OpenResource {
        shelf_id: u64,
        resource_id: u64,
    },
    AddResources {
        shelf_id: u64,
        selections: Vec<ResourceSelection>,
    },
    RemoveResource {
        shelf_id: u64,
        resource_id: u64,
    },
    Clear {
        shelf_id: u64,
    },
    DeleteShelf(u64),
    GetOrCreateShelf {
        shelf_id: u64,
    },
    CreateAndPasteFromClipboard {
        shelf_id: u64,
    },

    #[serde(skip)]
    ShelfLoaded(Shelf),
    #[serde(skip)]
    ShelfCreated(Shelf),
    #[serde(skip)]
    ShelfUpdated(Shelf),
    #[serde(skip)]
    ShelfDeleted(u64),
    #[serde(skip)]
    ModelEvent(LocalResourceEvent),
}

pub struct ShelfModule;

impl AppModule<BitBridge> for ShelfModule {
    type Event = ShelfEvent;
    type ViewModel = ShelfViewModel;

    fn update(
        &self,
        event: Self::Event,
        model: &mut AppModel,
        _caps: &<BitBridge as App>::Capabilities,
    ) -> Command<<BitBridge as App>::Effect, <BitBridge as App>::Event> {
        match event {
            Self::Event::Launch => Command::handle_result(|it| async move { it.app().load_shelves().await }),
            Self::Event::BeginLoadingResources => {
                model.shelf.is_loading = true;
                Command::render()
            }
            Self::Event::EndLoadingResources => {
                model.shelf.is_loading = false;
                Command::render()
            }
            Self::Event::ValidateLoadedResources(resource_ids) => {
                Command::handle_result(move |it| async move { it.app().validate_loaded_resources(resource_ids).await })
            }
            Self::Event::AddResources { shelf_id, selections } => {
                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::operate(DialogOperation::Toast(format!("Shelf not found {shelf_id:?}")));
                };

                let mut commands = vec![];
                let mut filtered_selections = vec![];
                for selection in selections {
                    if shelf.is_exists(&selection.path) {
                        commands.push(Command::operate(DialogOperation::Toast(
                            "Resource was already added before.".to_owned(),
                        )))
                    } else {
                        filtered_selections.push(selection);
                    }
                }

                commands.push(Command::handle_result(move |it| async move {
                    let app = it.app();
                    app.notify_event(ShelfEvent::BeginLoadingResources);
                    app.new_resources(shelf_id, filtered_selections).await?;
                    app.notify_event(ShelfEvent::EndLoadingResources);

                    Ok(())
                }));

                Command::all(commands)
            }
            Self::Event::RemoveResource { shelf_id, resource_id } => {
                if model.transfer.has_active_send_session(shelf_id) {
                    return Command::operate(DialogOperation::Toast(
                        "Cannot remove resource during active transfer.".to_owned(),
                    ));
                }

                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::operate(DialogOperation::Toast("Shelf not found.".to_owned()));
                };

                let Some(resource) = shelf.get(&LocalResourceId {
                    order_id: Some(resource_id),
                    shelf_id: Some(shelf_id),
                    ..Default::default()
                }) else {
                    return Command::operate(DialogOperation::Toast("Resource not found.".to_owned()));
                };

                let mut id = resource.id();
                id.shelf_id = Some(shelf_id);
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
                        if let Some(shelf_id) = id.shelf_id {
                            if let Some(shelf) = model.shelf.get_shelf_mut(shelf_id) {
                                shelf.remove_resource(&id);
                            }
                        }
                    }
                    LocalResourceEvent::Update(id, LocalResourceUpdateEvent::Update(resource)) => {
                        if let Some(shelf_id) = id.shelf_id {
                            if let Some(shelf) = model.shelf.get_shelf_mut(shelf_id) {
                                shelf.update_resource(&id, resource);
                            }
                        }
                    }
                }

                Command::done()
            }
            Self::Event::OpenResource { shelf_id, resource_id } => {
                let id = LocalResourceId {
                    order_id: Some(resource_id),
                    shelf_id: Some(shelf_id),
                    ..Default::default()
                };

                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::operate(DialogOperation::Toast("Shelf not found.".to_owned()));
                };

                let Some(resource) = shelf.get(&id) else {
                    return Command::operate(DialogOperation::Toast("Resource not found.".to_owned()));
                };

                let resource_path = resource.path.clone();
                Command::new(move |it| async move {
                    let _ = DeviceOperation::open(resource_path).into_future(it.clone()).await;
                })
            }
            Self::Event::Clear { shelf_id } => {
                let Some(shelf) = model.shelf.get_shelf(shelf_id) else {
                    return Command::operate(DialogOperation::Toast("Shelf not found.".to_owned()));
                };

                let commands = shelf
                    .resources
                    .iter()
                    .map(|it| {
                        let mut id = it.id();
                        id.shelf_id = Some(shelf_id);
                        Command::handle_result(move |it| async move {
                            let _ = it.app().remove_resource(id).await;
                            Ok(())
                        })
                    })
                    .collect::<Vec<_>>();

                Command::all(commands)
            }
            Self::Event::ShelfCreated(shelf) => {
                model.shelf.shelves.insert(0, shelf);
                Command::render()
            }
            Self::Event::GetOrCreateShelf { shelf_id } => {
                if model.shelf.get_shelf(shelf_id).is_some() {
                    return Command::done();
                }

                if is_shelf_limit_reached(model) {
                    return Command::operate(DeviceOperation::ShowUpgradeDialog(shelf_id));
                }

                Command::handle_result(move |it| async move {
                    it.app().create_shelf(shelf_id).await
                })
            }
            Self::Event::CreateAndPasteFromClipboard { shelf_id } => {
                let shelf_exists = model.shelf.get_shelf(shelf_id).is_some();

                if !shelf_exists && is_shelf_limit_reached(model) {
                    return Command::operate(DeviceOperation::ShowUpgradeDialog(shelf_id));
                }

                Command::new(move |it| async move {
                    if !shelf_exists {
                        let _ = it.app().create_shelf(shelf_id).await;
                    }
                    let selections = DeviceOperation::paste_clipboard(shelf_id).into_future(it.clone()).await;
                    if !selections.is_empty() {
                        it.app().notify_event(ShelfEvent::AddResources { shelf_id, selections });
                    }
                })
            }
            Self::Event::DeleteShelf(shelf_id) => {
                if model.shelf.shelves.len() <= 1 {
                    return Command::operate(DialogOperation::Toast("Cannot delete the last shelf.".to_owned()));
                }

                Command::handle_result(move |it| async move {
                    it.app().delete_shelf(shelf_id).await?;
                    it.app().notify_event(ShelfEvent::ShelfDeleted(shelf_id));
                    Ok(())
                })
            }
            Self::Event::ShelfDeleted(shelf_id) => {
                model.shelf.shelves.retain(|s| s.id != shelf_id);
                Command::render()
            }
            Self::Event::ShelfLoaded(shelf) => {
                log::info!("Shelf loaded: {:?}", shelf.resources.len());
                model.shelf.shelves.push(shelf);
                Command::done()
            }
            Self::Event::ShelfUpdated(shelf) => {
                if let Some(existing) = model.shelf.get_shelf_mut(shelf.id) {
                    existing.name = shelf.name;
                }

                Command::render()
            }
        }
    }

    fn view(&self, model: &AppModel) -> Self::ViewModel {
        let shelf_limit = model
            .authentication
            .capabilities
            .as_ref()
            .and_then(|c| c.shelf_limit())
            .map(|n| n as usize);

        let unlocked_ids = shelf_limit
            .map(|limit| compute_unlocked_shelf_ids(&model.shelf.shelves, limit))
            .unwrap_or_default();

        let mut shelves: Vec<ShelfItemViewModel> = model
            .shelf
            .shelves
            .iter()
            .map(|shelf| {
                let active_session = model.transfer.get_active_p2p_send_session(shelf.id);
                let is_online = active_session.is_some();
                let mut view_model = ShelfItemViewModel::from_shelf(shelf, is_online);
                view_model.is_resource_remove_allowed = !model.transfer.has_active_send_session(shelf.id);

                if shelf_limit.is_some() && !unlocked_ids.contains(&shelf.id) {
                    view_model.is_locked = true;
                    view_model.lock_reason = Some("Upgrade to unlimited plan".to_owned());
                }

                if let Some(session) = active_session {
                    for resource_vm in &mut view_model.resources {
                        if let Ok(order_id) = resource_vm.order_id.parse::<u64>() {
                            if let Some(progress) = session.resource_progress(order_id) {
                                resource_vm.received_by_peers = if progress.is_failed() || progress.is_canceled() {
                                    Vec::new()
                                } else {
                                    progress.received_by_peers().iter().map(PeerAvatarViewModel::from).collect()
                                };
                            }
                        }
                    }
                }

                view_model
            })
            .collect();

        shelves.sort_by(|a, b| b.id.cmp(&a.id));

        ShelfViewModel {
            shelves,
            is_loading: model.shelf.is_loading,
        }
    }
}

fn is_shelf_limit_reached(model: &AppModel) -> bool {
    model
        .authentication
        .capabilities
        .as_ref()
        .and_then(|c| c.shelf_limit())
        .map(|limit| model.shelf.shelves.len() >= limit as usize)
        .unwrap_or(false)
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ResourceSelection {
    pub path: LocalResourcePath,
    // This is optional, if it is None, we will detect by Rust code to see if it should be a Folder or a File
    pub r#type: Option<ResourceType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shelf_with_id(id: u64) -> Shelf {
        Shelf::with_id(id, format!("shelf-{id}"))
    }

    #[test]
    fn free_user_with_two_shelves_locks_the_newer_one() {
        let old = shelf_with_id(100);
        let new = shelf_with_id(200);
        let unlocked = compute_unlocked_shelf_ids(&[old.clone(), new.clone()], 1);
        assert!(unlocked.contains(&old.id), "older shelf (smaller id) must stay unlocked");
        assert!(!unlocked.contains(&new.id), "newer shelf (larger id) must be locked");
    }

    #[test]
    fn stored_order_does_not_affect_lock_selection() {
        let older = shelf_with_id(100);
        let newer = shelf_with_id(200);
        let unlocked_a = compute_unlocked_shelf_ids(&[older.clone(), newer.clone()], 1);
        let unlocked_b = compute_unlocked_shelf_ids(&[newer.clone(), older.clone()], 1);
        assert_eq!(unlocked_a, unlocked_b);
        assert_eq!(unlocked_a, [100u64].into_iter().collect());
    }

    #[test]
    fn limit_larger_than_count_unlocks_everything() {
        let a = shelf_with_id(10);
        let b = shelf_with_id(20);
        let unlocked = compute_unlocked_shelf_ids(&[a, b], 5);
        assert_eq!(unlocked.len(), 2);
    }

    #[test]
    fn limit_of_two_keeps_two_oldest_unlocked() {
        let s1 = shelf_with_id(1);
        let s2 = shelf_with_id(2);
        let s3 = shelf_with_id(3);
        let unlocked = compute_unlocked_shelf_ids(&[s3.clone(), s1.clone(), s2.clone()], 2);
        assert!(unlocked.contains(&1));
        assert!(unlocked.contains(&2));
        assert!(!unlocked.contains(&3));
    }
}
