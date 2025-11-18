use crate::cloud_storage::storage::CloudStorage;
use crate::entities::transfer_progress::TransferProgress;
use crate::entities::transfer_resource::{TransferResource, TransferResourceType};
use crate::entities::transfer_session::TransferSession;
use schema::devlog::app_gateway::models::Application;
use schema::devlog::bitbridge::cloud_resource_message::ResourceType as CloudResourceType;
use schema::devlog::bitbridge::public_transfer_session_message::Progress;
use schema::devlog::bitbridge::subscribe_session_info_response::{Event, ProgressUpdated, ResourceUpdated, SessionUpdated};
use schema::devlog::bitbridge::{CloudResourceMessage, PublicTransferSessionMessage, ResourceTypeMessage};
use std::sync::Arc;

impl From<&CloudResourceType> for TransferResourceType {
    fn from(value: &CloudResourceType) -> Self {
        match value {
            CloudResourceType::Image => Self::Image,
            CloudResourceType::Video => Self::Video,
            CloudResourceType::File => Self::File,
            CloudResourceType::Folder => Self::Folder
        }
    }
}

impl From<TransferProgress> for Progress {
    fn from(value: TransferProgress) -> Self {
        Self {
            resource_order_id: value.resource_id(),
            total_size: value.size(),
            transfered_amount: value.transfered_amount(),
            error_message: value.error_message().map(|it| it.to_owned())
        }
    }
}

impl From<TransferResourceType> for ResourceTypeMessage {
    fn from(value: TransferResourceType) -> Self {
        match value {
            TransferResourceType::File => Self::File,
            TransferResourceType::Folder => Self::Folder,
            TransferResourceType::Image => Self::Image,
            TransferResourceType::Video => Self::Video
        }
    }
}

impl TransferResource {
    pub async fn into_resource_msg(&self, cloud_storage: &Arc<dyn CloudStorage>) -> CloudResourceMessage {
        let mut source = self.source();
        let thumbnail_source = self.thumbnail_source();
        let download_url = cloud_storage.generate_download_url(&mut source).await.unwrap_or_default();

        let download_thumbnail_url = match thumbnail_source {
            Some(mut thumbnail_source) => cloud_storage.generate_download_url(&mut thumbnail_source).await.ok(),
            None => None
        };

        CloudResourceMessage {
            r#type: Into::<ResourceTypeMessage>::into(self.r#type()).into(),
            name: self.name().to_owned(),
            order_id: self.order_id(),
            size: self.size_in_bytes() as i64,
            thumbnail_download_url: download_thumbnail_url,
            download_url
        }
    }
}

impl TransferSession {
    pub async fn into_msg(&self, cloud_storage: &Arc<dyn CloudStorage>, app: &Application) -> PublicTransferSessionMessage {
        let mut resources = vec![];
        for resource in self.resources() {
            let _ = self.progresses().iter().find(|it| it.resource_id() == resource.order_id()).unwrap();
            resources.push(resource.clone().into_resource_msg(cloud_storage).await);
        }

        let msg = PublicTransferSessionMessage {
            order_id: self.order_id(),
            user_id: self.user_order_id(),
            password: self.password(),
            access_url: self.access_url(app.web_url().to_owned()),
            resources,
            progresses: self.progresses().iter().map(|it| it.clone().into()).collect(),
            to_emails: self.to_emails().clone()
        };

        msg
    }

    pub async fn get_change_events(
        &self,
        new: &TransferSession,
        cloud_storage: &Arc<dyn CloudStorage>,
        app: &Application
    ) -> Vec<Event> {
        let mut events = vec![];

        let resource_changes = new
            .resources()
            .iter()
            .zip(self.resources().iter())
            .filter(|(new, curr)| new.ne(curr))
            .map(|(new, _)| new)
            .collect::<Vec<_>>();

        if !resource_changes.is_empty() {
            let mut resources = vec![];
            for resource in resource_changes {
                let Some(_) = new.progresses().iter().find(|it| it.resource_id() == resource.order_id()) else {
                    continue;
                };

                resources.push(resource.clone().into_resource_msg(cloud_storage).await);
            }

            events.push(Event::ResourceUpdated(ResourceUpdated {
                resource_update: resources
            }));
        }

        let progress_changes = new
            .progresses()
            .iter()
            .zip(self.progresses().iter())
            .filter(|(new, curr)| new.ne(curr) || new.status() != curr.status() || new.transfered_amount() != curr.transfered_amount())
            .map(|(new, _)| new)
            .collect::<Vec<_>>();

        if !progress_changes.is_empty() {
            events.push(Event::ProgressUpdated(ProgressUpdated {
                progress_update: progress_changes.iter().map(|it| (*it).clone().into()).collect()
            }));
        }

        if !events.is_empty() {
            return events;
        }

        vec![
            Event::SessionUpdated(SessionUpdated {
                session_updated: new.into_msg(cloud_storage, app).await
            }),
        ]
    }
}
