use crate::entities::local_resource::{LocalResource, LocalResourcePath};
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::{TransferProgress, TransferSession, TransferType};
use crate::repository::errors::PersistenceError;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferTargetId {
    Internet,
    P2P
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TransferSessionId {
    pub transfer_type: Option<TransferType>,
    pub order_id: Option<String>
}

impl From<&TransferTarget> for TransferTargetId {
    fn from(value: &TransferTarget) -> Self {
        match value {
            TransferTarget::Internet { .. } => TransferTargetId::Internet,
            TransferTarget::P2P { .. } => TransferTargetId::P2P
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZipDownloadPaths {
    pub resource_paths: HashMap<u64, LocalResourcePath>,
    pub session_path: LocalResourcePath
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait TransferSessionRepository: Repository<TransferSession, TransferSessionId> {
    async fn update_progresses(
        &self,
        order_id: u64,
        progresses: Vec<TransferProgress>
    ) -> Result<Option<TransferSession>, PersistenceError>;
    async fn update_resource(
        &self,
        session_id: TransferSessionId,
        resource: LocalResource
    ) -> Result<Option<TransferSession>, PersistenceError>;

    async fn delete_session(&self, session_id: TransferSessionId) -> Result<(), PersistenceError>;
    async fn generate_resource_saved_paths(
        &self,
        session_order_id: u64,
        resource_names: HashMap<u64, String>
    ) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError>;

    async fn generate_zip_download_paths(
        &self,
        session_order_id: u64,
        resource_names: HashMap<u64, String>
    ) -> Result<ZipDownloadPaths, PersistenceError>;

    async fn start_download_session(&self, zip_path: LocalResourcePath) -> Result<(), PersistenceError>;

    async fn stop_download_session(&self, zip_path: LocalResourcePath) -> Result<(), PersistenceError>;
}

impl Table<TransferSessionId> for TransferSession {
    fn get_table() -> &'static str {
        "transferSession"
    }

    fn id(&self) -> TransferSessionId {
        TransferSessionId {
            transfer_type: Some(self.transfer_type.clone()),
            order_id: Some(self.order_id.to_string())
        }
    }
}

impl DbId for TransferSessionId {
    type Table = TransferSession;

    fn is_represent(&self, table: &Self::Table) -> bool {
        if let Some(transfer_type) = &self.transfer_type {
            if transfer_type != &table.transfer_type {
                return false;
            }
        }

        if let Some(order_id) = &self.order_id {
            let order_id: u64 = order_id.parse().unwrap_or_default();
            if order_id != table.order_id {
                return false;
            }
        }

        true
    }
}
