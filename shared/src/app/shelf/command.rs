use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::core::model_events::LocalResourceEvent;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::LocalResourcePersistentOperation;
use crate::app::shelf::module::ResourceSelection;
use crate::app::AppEvent;
use crate::errors::CoreError;
use crate::repository::local_resource::LocalResourceId;

impl AppCommand {
    pub async fn load_resources(&self) -> Result<(), CoreError> {
        let resources = LocalResourcePersistentOperation::find_all().into_future(self.ctx()).await?;
        let model_events = resources
            .into_iter()
            .map(|it| Into::<AppEvent>::into(LocalResourceEvent::Add(it)))
            .collect::<Vec<_>>();
        self.update_model_series(model_events);
        Ok(())
    }

    pub async fn new_resources(&self, mut selections: Vec<ResourceSelection>) -> Result<(), CoreError> {
        while let Some(selection) = selections.pop() {
            let Some(mut local_resource) = self.run(LocalResourcePersistentOperation::load_from_disk(selection.path.clone())).await?
            else {
                log::error!("File not exists: {:?}", selection.path);
                continue;
            };

            local_resource.path = selection.path.clone();
            local_resource.r#type = match selection.r#type.clone() {
                Some(r#type) => r#type,
                None => self.run(LocalResourcePersistentOperation::get_resource_type(selection.path.clone())).await?
            };

            let (thumbnail_png_opt, thumbnail_path_opt) = self.run(
                DeviceOperation::load_thumbnail_png(local_resource.order_id, selection.path.clone(), local_resource.r#type.clone())).await;

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
            if new_resources.is_empty() {
                continue;
            }

            let new_resource = new_resources.pop().unwrap();

            self.update_model(LocalResourceEvent::Add(new_resource));
        }

        Ok(())
    }

    pub async fn remove_resource(&self, id: u64) -> Result<(), CoreError> {
        let removed = self.run(LocalResourcePersistentOperation::remove(id)).await?;
        if removed {
            self.update_model(LocalResourceEvent::Remove(LocalResourceId {
                order_id: Some(id),
                ..Default::default()
            }));
        }

        Ok(())
    }
}
