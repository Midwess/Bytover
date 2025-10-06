use crate::entities::local_resource::LocalResource;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferProgress, TransferSession};
use crate::repository::local_resource::LocalResourceId;
use crate::repository::transfer_session::TransferSessionId;
use ambassador::{delegatable_trait, Delegate};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use crate::app::AppEvent;
use crate::app::transfer::module::TransferEvent;
use crate::app::shelf::module::ShelfEvent;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ModelEvent<D, I, U> {
    Update(I, U),
    Add(D),
    Remove(I)
}

#[delegatable_trait]
pub trait UpdateAction<Data> {
    fn update(self, data: &mut Data);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Delegate, From)]
#[delegate(UpdateAction<TransferSession>)]
pub enum TransferSessionUpdateEvent {
    ProgressUpdate(TransferProgress),
    ThumbnailUpdated(ThumbnailUpdatedEvent)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From)]
pub enum LocalResourceUpdateEvent {
    Update
}

pub type LocalResourceEvent = ModelEvent<LocalResource, LocalResourceId, LocalResourceUpdateEvent>;
pub type TransferSessionModelEvent = ModelEvent<TransferSession, TransferSessionId, TransferSessionUpdateEvent>;

impl Into<AppEvent> for TransferSessionModelEvent {
    fn into(self) -> AppEvent {
        TransferEvent::ModelEvent(self).into()
    }
}

impl Into<AppEvent> for LocalResourceEvent {
    fn into(self) -> AppEvent {
        ShelfEvent::ModelEvent(self).into()
    }
}
