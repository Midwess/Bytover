use crate::app::shelf::module::ShelfEvent;
use crate::app::transfer::module::TransferEvent;
use crate::app::AppEvent;
use crate::entities::local_resource::LocalResource;
use crate::entities::transfer_session::{ThumbnailUpdatedEvent, TransferProgress, TransferSession};
use crate::repository::local_resource::LocalResourceId;
use crate::repository::transfer_session::TransferSessionId;
use ambassador::{delegatable_trait, Delegate};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionLoadError(pub String);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ModelEvent<D, I, U> {
    Update(I, U),
    Add(D),
    Remove(I),
}

#[delegatable_trait]
pub trait UpdateAction<Data> {
    fn update(self, data: &mut Data);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Delegate, From)]
#[delegate(UpdateAction<TransferSession>)]
pub enum TransferSessionUpdateEvent {
    ProgressUpdate(TransferProgress),
    ThumbnailUpdated(ThumbnailUpdatedEvent),
    ResourceUpdate(LocalResource),
    SessionDetailUpdated(schema::devlog::bitbridge::P2pTransferSessionMessage),
    SessionLoadError(SessionLoadError)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From)]
pub enum LocalResourceUpdateEvent {
    Update
}

pub type LocalResourceEvent = ModelEvent<LocalResource, LocalResourceId, LocalResourceUpdateEvent>;
pub type TransferSessionModelEvent = ModelEvent<TransferSession, TransferSessionId, TransferSessionUpdateEvent>;

impl From<TransferSessionModelEvent> for AppEvent {
    fn from(val: TransferSessionModelEvent) -> Self {
        TransferEvent::ModelEvent(val).into()
    }
}

impl From<LocalResourceEvent> for AppEvent {
    fn from(val: LocalResourceEvent) -> Self {
        ShelfEvent::ModelEvent(val).into()
    }
}
