use devlog_sdk::distributed_id::gen_id;
use serde::{Deserialize, Serialize};
use surreal_derive_plus::SurrealDerive;
use uniffi::{Enum, Record};

use super::file::LocalResource;
use super::user::User;

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct TransferSession {
    pub order_id: u64,
    pub resources: Vec<LocalResource>,
    pub processes: Vec<TransferProcess>
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferTarget {
    User(User),
    Device(String)
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferMethod {
    Internet(InternetTransfer),
    LocalNetwork(LocalNetworkTransfer)
}

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct InternetTransfer {}

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct LocalNetworkTransfer {
    // Device or user within local network
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferProcessStatus {
    Fail,
    InProgress,
    Success
}

#[derive(Debug, PartialEq, Eq, Enum, Serialize, Deserialize, Clone, SurrealDerive)]
pub enum TransferSessionStatus {
    New,
    Fail,
    Transfering,
    Success
}

#[derive(Debug, PartialEq, Eq, Record, Serialize, Deserialize, Clone, SurrealDerive)]
pub struct TransferProcess {
    status: TransferProcessStatus,
    method: TransferMethod
}

impl TransferSession {
    pub async fn new() -> Self {
        Self {
            order_id: gen_id().await,
            resources: vec![],
            processes: vec![]
        }
    }

    pub fn transfer_status(&self) -> TransferSessionStatus {
        if self.processes.is_empty() {
            return TransferSessionStatus::New
        }

        if self.processes.iter().any(|p| p.status == TransferProcessStatus::InProgress) {
            return TransferSessionStatus::Transfering
        }

        if self.processes.iter().all(|p| p.status == TransferProcessStatus::Success) {
            return TransferSessionStatus::Success
        }

        TransferSessionStatus::Fail
    }

    pub fn add_resource(&mut self, resource: LocalResource) {
        self.resources.push(resource);
    }
}
