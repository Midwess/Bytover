use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::core::model_events::LocalResourceEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::{LocalResourcePersistentOperation, ShelfPersistentOperation};
use crate::app::shelf::module::{ResourceSelection, ShelfEvent};
use crate::app::transfer::module::TransferEvent;
use crate::entities::shelf::Shelf;
use crate::errors::CoreError;
use crate::repository::local_resource::LocalResourceId;

impl AppCommand {
    pub async fn load_shelves(&self) -> Result<(), CoreError> {
        let mut shelves = ShelfPersistentOperation::find_all().into_future(self.ctx()).await?;
        shelves.sort_by(|a, b| b.id.cmp(&a.id));
        log::info!("Loaded {} shelves", shelves.len());

        let resources = LocalResourcePersistentOperation::find_all().into_future(self.ctx()).await?;
        log::info!("Loaded {} resources", resources.len());

        for shelf in shelves {
            self.notify_event(ShelfEvent::ShelfLoaded(shelf));
        }

        for resource in resources {
            let shelf_id = resource.shelf_id;
            self.update_model(LocalResourceEvent::Add {
                shelf_id,
                resource
            });
        }

        Ok(())
    }

    pub async fn create_shelf(&self, name: String) -> Result<Shelf, CoreError> {
        let shelf = Shelf::new(name);
        let saved_shelf = ShelfPersistentOperation::add(shelf).into_future(self.ctx()).await?;
        Ok(saved_shelf)
    }

    pub async fn delete_shelf(&self, shelf_id: u64) -> Result<bool, CoreError> {
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
                None => self.run(LocalResourcePersistentOperation::get_resource_type(selection.path.clone())).await?
            };

            let (thumbnail_png_opt, thumbnail_path_opt) = self
                .run(DeviceOperation::load_thumbnail_png(
                    local_resource.order_id,
                    selection.path.clone(),
                    local_resource.r#type.clone()
                ))
                .await;

            if let Some(thumbnail_png) = thumbnail_png_opt {
                match self
                    .run(LocalResourcePersistentOperation::add_thumbnail(
                        thumbnail_png,
                        local_resource.order_id
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
                resource: new_resource.clone()
            });
            self.notify_event(TransferEvent::NewTransferResource {
                shelf_id: target_shelf_id,
                resource: new_resource
            });
        }

        Ok(())
    }

    pub async fn remove_resource(&self, local_resource_id: LocalResourceId) -> Result<(), CoreError> {
        let Some(path) = local_resource_id.path.clone() else {
            return Ok(());
        };

        let removed = self.run(LocalResourcePersistentOperation::remove(path)).await?;
        if removed {
            self.update_model(LocalResourceEvent::Remove(local_resource_id));
        }

        Ok(())
    }
}
