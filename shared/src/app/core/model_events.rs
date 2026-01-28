use crate::app::shelf::module::ShelfEvent;
use crate::app::transfer::module::TransferEvent;
use crate::app::AppEvent;
use crate::entities::local_resource::LocalResource;
use crate::entities::transfer_session::{SessionResourceUpdate, ThumbnailUpdatedEvent, TransferProgress, TransferSession};
use crate::repository::local_resource::LocalResourceId;
use crate::repository::transfer_session::TransferSessionId;
use ambassador::{delegatable_trait, Delegate};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionLoadError(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ConnectionRecovered;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerReceivedEvent {
    pub resource_order_id: u64,
    pub peer_id: String
}

impl UpdateAction<TransferSession> for PeerReceivedEvent {
    fn update(self, data: &mut TransferSession) {
        if let Some(progress) = data.resource_mut_progress(self.resource_order_id) {
            progress.mark_received_by_peer(self.peer_id);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Delegate, From)]
#[delegate(UpdateAction<TransferSession>)]
pub enum TransferSessionUpdateEvent {
    ProgressUpdate(TransferProgress),
    ThumbnailUpdated(ThumbnailUpdatedEvent),
    ResourceUpdate(LocalResource),
    SessionResourceUpdate(SessionResourceUpdate),
    SessionDetailUpdated(schema::devlog::bitbridge::P2pTransferSessionMessage),
    SessionLoadError(SessionLoadError),
    ConnectionRecovered(ConnectionRecovered),
    PeerReceived(PeerReceivedEvent)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, From)]
pub enum LocalResourceUpdateEvent {
    Update
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocalResourceEvent {
    Add { shelf_id: u64, resource: LocalResource },
    Remove(LocalResourceId),
    Update(LocalResourceId, LocalResourceUpdateEvent)
}

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
