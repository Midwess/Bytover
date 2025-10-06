use crate::app::core::command::AppCommand;
use crate::app::core::model_events::LocalResourceEvent;
use crate::app::core_utils::CoreCommandContextUtils;
use crate::app::operations::device::DeviceOperation;
use crate::app::operations::persistent::LocalResourcePersistentOperation;
use crate::app::shelf::module::{ResourceSelection, ShelfEvent};
use crate::app::AppEvent;
use crate::repository::local_resource::LocalResourceId;

impl AppCommand {
    pub async fn load_resources(&self) {
        let resources = LocalResourcePersistentOperation::find_all().into_future(self.ctx()).await;
        let model_events = resources.into_iter().map(LocalResourceEvent::Add).collect::<Vec<_>>();
        self.update_model(AppEvent::Shelf(ShelfEvent::ResourceModelEvents(model_events)));
    }

    pub async fn new_resources(&self, mut selections: Vec<ResourceSelection>) {
        while let Some(selection) = selections.pop() {
            let Some(mut local_resource) = self.run(LocalResourcePersistentOperation::load_from_disk(selection.path.clone())).await
            else {
                log::error!("File not exists: {:?}", selection.path);
                continue;
            };

            local_resource.path = selection.path.clone();
            local_resource.r#type = match selection.r#type.clone() {
                Some(r#type) => r#type,
                None => self.run(LocalResourcePersistentOperation::get_resource_type(selection.path.clone())).await
            };

            let mut new_resources = self.run(LocalResourcePersistentOperation::add(vec![local_resource])).await;
            if new_resources.is_empty() {
                log::info!("File already exists: {:?}", selection.path);
                continue;
            }

            let mut new_resource = new_resources.pop().unwrap();

            match self.run(DeviceOperation::load_thumbnail_png(selection.path.clone())).await {
                (Some(thumbnail_png), _) => {
                    let thumbnail = self
                        .run(LocalResourcePersistentOperation::add_thumbnail(
                            thumbnail_png,
                            new_resource.order_id
                        ))
                        .await;
                    new_resource.thumbnail_path = Some(thumbnail);
                }
                (_, Some(thumbnail_path)) => {
                    new_resource.thumbnail_path = Some(thumbnail_path);
                }
                _ => {}
            };

            self.update_model(ShelfEvent::ResourceModelEvents(vec![LocalResourceEvent::Add(new_resource)]));
        }
    }

    pub async fn remove_resource(&self, id: u64) {
        let removed = self.run(LocalResourcePersistentOperation::remove(id)).await;
        if removed {
            self.update_model(ShelfEvent::ResourceModelEvents(vec![
                LocalResourceEvent::Remove(LocalResourceId {
                    order_id: Some(id),
                    ..Default::default()
                }),
            ]));
        }
    }
}
