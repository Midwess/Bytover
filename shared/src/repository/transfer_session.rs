use crate::entities::local_resource::{LocalResource, LocalResourcePath};
use crate::entities::target::TransferTarget;
use crate::entities::transfer_session::{TransferProgress, TransferSession, TransferType};
use crate::repository::errors::PersistenceError;
use core_services::db::repository::abstraction::id::DbId;
use core_services::db::repository::abstraction::repository::Repository;
use core_services::db::repository::abstraction::table::Table;
use frunk::Generic;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferTargetId {
    Internet,
    Nearby
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TransferSessionId {
    pub r#type: Option<TransferType>,
    pub target: Option<TransferTargetId>,
    pub order_id: Option<u64>
}

impl From<&TransferTarget> for TransferTargetId {
    fn from(value: &TransferTarget) -> Self {
        match value {
            TransferTarget::Internet { .. } => TransferTargetId::Internet,
            TransferTarget::Nearby(_) => TransferTargetId::Nearby
        }
    }
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
    async fn generate_resource_paths(
        &self,
        session_order_id: u64,
        resource_names: HashMap<u64, String>
    ) -> Result<HashMap<u64, LocalResourcePath>, PersistenceError>;
}

impl Table<TransferSessionId> for TransferSession {
    fn get_table() -> &'static str {
        "transferSession"
    }

    fn id(&self) -> TransferSessionId {
        TransferSessionId {
            r#type: Some(self.transfer_type.clone()),
            target: Some((&self.target).into()),
            order_id: Some(self.order_id)
        }
    }
}

impl DbId for TransferSessionId {
    type Table = TransferSession;

    fn is_represent(&self, table: &Self::Table) -> bool {
        if let Some(r#type) = &self.r#type {
            if r#type != &table.transfer_type {
                return false;
            }
        }

        if let Some(target) = &self.target {
            let table_target_id: TransferTargetId = (&table.target).into();
            if target != &table_target_id {
                return false;
            }
        }

        if let Some(order_id) = &self.order_id {
            if order_id != &table.order_id {
                return false;
            }
        }

        true
    }
}
