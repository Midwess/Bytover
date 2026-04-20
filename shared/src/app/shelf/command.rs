use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::core::model_events::{LocalResourceEvent, LocalResourceUpdateEvent};
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::{DeviceAliasPersistentOperation, LocalResourcePersistentOperation, ShelfPersistentOperation};
use crate::app::operations::rpc::RpcOperation;
use crate::app::shelf::module::{ResourceSelection, ShelfEvent};
use crate::app::transfer::module::TransferEvent;
use crate::entities::local_resource::LocalResource;
use crate::entities::shelf::Shelf;
use crate::errors::CoreError;
use crate::repository::local_resource::LocalResourceId;
use crate::{gen_shelf_id, CoreOperation};

pub const MAX_SHELVES: usize = 10;

impl AppCommand {
    pub async fn load_shelves(&self) -> Result<(), CoreError> {
        let shelves = ShelfPersistentOperation::find_all(Some(10)).into_future(self.ctx()).await?;
        log::info!("Loaded {} shelves", shelves.len());

        // Ensure there's always at least one shelf
        if shelves.is_empty() {
            log::info!("No shelves found, creating default shelf");
            self.create_shelf(gen_shelf_id()).await?;
        }

        let resources = LocalResourcePersistentOperation::find_all().into_future(self.ctx()).await?;
        log::info!("Loaded {} resources", resources.len());
        let validation_resources = resources.clone();

        for shelf in shelves {
            self.update_model(ShelfEvent::ShelfLoaded(shelf));
        }

        for resource in resources {
            let shelf_id = resource.shelf_id;
            self.update_model(LocalResourceEvent::Add { shelf_id, resource });
        }

        self.notify_shell(CoreOperation::Render);
        if !validation_resources.is_empty() {
            self.notify_event(ShelfEvent::ValidateLoadedResources(validation_resources));
        }

        Ok(())
    }

    pub async fn validate_loaded_resources(&self, resources: Vec<LocalResource>) -> Result<(), CoreError> {
        let _ = self.sync_local_resources(resources).await;
        Ok(())
    }

    pub async fn sync_local_resources(&self, resources: Vec<LocalResource>) -> Vec<LocalResource> {
        let mut synced_resources = Vec::with_capacity(resources.len());

        for current_resource in resources {
            let resource_id = LocalResourceId {
                order_id: Some(current_resource.order_id),
                path: Some(current_resource.path.clone()),
                shelf_id: Some(current_resource.shelf_id),
            };

            let loaded_resource = match self.run(LocalResourcePersistentOperation::load_from_disk(current_resource.path.clone())).await
            {
                Ok(resource) => resource,
                Err(error) => {
                    log::warn!("Failed to reload resource {:?}: {:?}", resource_id, error);
                    synced_resources.push(current_resource);
                    continue;
                }
            };

            let Some(mut loaded_resource) = loaded_resource else {
                log::info!("Removing missing resource {:?}", resource_id);
                if let Err(error) = self.remove_resource(resource_id.clone()).await {
                    log::error!("Failed to remove missing resource {:?}: {:?}", resource_id, error);
                }
                continue;
            };

            loaded_resource.order_id = current_resource.order_id;
            loaded_resource.path = current_resource.path.clone();
            loaded_resource.thumbnail_path = current_resource.thumbnail_path.clone();
            loaded_resource.shelf_id = current_resource.shelf_id;

            if loaded_resource == current_resource {
                synced_resources.push(current_resource);
                continue;
            }

            let updated_resource = match self.run(LocalResourcePersistentOperation::update(loaded_resource.clone())).await {
                Ok(resource) => resource,
                Err(error) => {
                    log::error!("Failed to update resource {:?}: {:?}", resource_id, error);
                    synced_resources.push(loaded_resource);
                    continue;
                }
            };

            self.update_model(LocalResourceEvent::Update(
                resource_id.clone(),
                LocalResourceUpdateEvent::Update(updated_resource.clone()),
            ));
            synced_resources.push(updated_resource);
        }

        synced_resources
    }

    pub async fn create_shelf(&self, id: u64) -> Result<(), CoreError> {
        let name = self.get_next_shelf_alias().await;
        log::info!("Creating shelf {id}; {name}");
        let shelf = Shelf::with_id(id, name);
        let saved_shelf = ShelfPersistentOperation::add(shelf).into_future(self.ctx()).await?;
        self.update_model(ShelfEvent::ShelfCreated(saved_shelf));
        Ok(())
    }

    async fn get_next_shelf_alias(&self) -> String {
        let mut aliases = DeviceAliasPersistentOperation::get_all().into_future(self.ctx()).await.unwrap_or_default();

        if aliases.is_empty() {
            if let Ok(fetched_aliases) = RpcOperation::get_device_aliases().into_future(self.ctx()).await {
                if !fetched_aliases.is_empty() {
                    let _ = DeviceAliasPersistentOperation::save_all(fetched_aliases.clone()).into_future(self.ctx()).await;
                    aliases = fetched_aliases;
                }
            }
        }

        let shelves = ShelfPersistentOperation::find_all(Some(MAX_SHELVES))
            .into_future(self.ctx())
            .await
            .unwrap_or_default();
        let used_names: Vec<&str> = shelves.iter().map(|s| s.name.as_str()).collect();

        for alias in &aliases {
            if !used_names.contains(&alias.as_str()) {
                return alias.clone();
            }
        }

        uuid::Uuid::new_v4().to_string()
    }

    pub async fn ensure_shelf_limit(&self, sessions: &[(u64, Option<u64>)]) -> Result<(), CoreError> {
        let shelves = ShelfPersistentOperation::find_all(Some(MAX_SHELVES + 1)).into_future(self.ctx()).await?;

        if shelves.len() < MAX_SHELVES {
            return Ok(());
        }

        let active_shelf_ids: Vec<u64> = sessions.iter().filter_map(|(_, shelf_id)| *shelf_id).collect();

        let mut sorted_shelves = shelves;
        sorted_shelves.sort_by_key(|s| s.id);

        for shelf in sorted_shelves {
            if !active_shelf_ids.contains(&shelf.id) {
                log::info!("Auto-removing shelf {} to make room for new shelf", shelf.id);
                self.delete_shelf(shelf.id).await?;
                self.notify_event(ShelfEvent::ShelfDeleted(shelf.id));
                DeviceOperation::close_shelf(shelf.id).into_future(self.ctx()).await;
                return Ok(());
            }
        }

        Ok(())
    }

    pub async fn delete_shelf(&self, shelf_id: u64) -> Result<bool, CoreError> {
        let resources = LocalResourcePersistentOperation::find_all().into_future(self.ctx()).await?;
        for resource in resources {
            if resource.shelf_id == shelf_id {
                let id = LocalResourceId {
                    order_id: Some(resource.order_id),
                    path: Some(resource.path.clone()),
                    shelf_id: Some(shelf_id),
                };
                let _ = self.remove_resource(id).await;
            }
        }

        let removed = ShelfPersistentOperation::remove(shelf_id).into_future(self.ctx()).await?;
        Ok(removed)
    }

    pub async fn new_resources(&self, target_shelf_id: u64, mut selections: Vec<ResourceSelection>) -> Result<(), CoreError> {
        while let Some(selection) = selections.pop() {
            let Some(mut local_resource) = self.run(LocalResourcePersistentOperation::load_from_disk(selection.path.clone())).await?
            else {
                log::error!("File not exists: {:?}", selection.path);
                continue;
            };

            local_resource.path = selection.path.clone();
            local_resource.shelf_id = target_shelf_id;
            local_resource.r#type = match selection.r#type.clone() {
                Some(r#type) => r#type,
                None => self.run(LocalResourcePersistentOperation::get_resource_type(selection.path.clone())).await?,
            };

            let (thumbnail_png_opt, thumbnail_path_opt) = self
                .run(DeviceOperation::load_thumbnail_png(
                    local_resource.order_id,
                    selection.path.clone(),
                    local_resource.r#type.clone(),
                ))
                .await;

            if let Some(thumbnail_png) = thumbnail_png_opt {
                match self
                    .run(LocalResourcePersistentOperation::add_thumbnail(
                        thumbnail_png,
                        local_resource.order_id,
                    ))
                    .await
                {
                    Ok(thumbnail) => {
                        local_resource.thumbnail_path = Some(thumbnail);
                    }
                    Err(e) => {
                        log::error!("Failed to add thumbnail: {:?}", e);
                    }
                }
            } else if let Some(thumbnail_path) = thumbnail_path_opt {
                local_resource.thumbnail_path = Some(thumbnail_path);
            }

            let mut new_resources = self.run(LocalResourcePersistentOperation::add(vec![local_resource])).await?;
            let Some(new_resource) = new_resources.pop() else {
                continue;
            };

            self.update_model(LocalResourceEvent::Add {
                shelf_id: target_shelf_id,
                resource: new_resource.clone(),
            });

            self.notify_event(TransferEvent::NewTransferResource {
                shelf_id: target_shelf_id,
                resource: new_resource,
            });
        }

        Ok(())
    }

    pub async fn remove_resource(&self, local_resource_id: LocalResourceId) -> Result<(), CoreError> {
        let Some(path) = local_resource_id.path.clone() else {
            return Ok(());
        };

        let Some(shelf_id) = local_resource_id.shelf_id else {
            return Ok(());
        };

        let removed = self.run(LocalResourcePersistentOperation::remove(path, shelf_id)).await?;
        if removed {
            self.update_model(LocalResourceEvent::Remove(local_resource_id));
        }

        Ok(())
    }
}
