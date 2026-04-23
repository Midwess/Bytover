use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plan {
    #[default]
    Free,
    Paid,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferLimits {
    pub password_encryption_allowed: bool,
    pub max_files_per_transfer: u32,
    pub total_transfer_bytes_lifetime_cap: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferUsage {
    pub total_transfer_bytes_used: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationLimits {
    pub max_visible_shelves: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserCapabilities {
    pub plan: Plan,
    pub transfer_limits: TransferLimits,
    pub transfer_usage: TransferUsage,
    pub presentation: PresentationLimits,
    pub capabilities_version: u32,
}

impl UserCapabilities {
    pub fn free_defaults() -> Self {
        Self {
            plan: Plan::Free,
            transfer_limits: TransferLimits {
                password_encryption_allowed: false,
                max_files_per_transfer: 10,
                total_transfer_bytes_lifetime_cap: 5 * 1024 * 1024 * 1024,
            },
            transfer_usage: TransferUsage::default(),
            presentation: PresentationLimits { max_visible_shelves: 1 },
            capabilities_version: 1,
        }
    }

    pub fn is_paid(&self) -> bool {
        matches!(self.plan, Plan::Paid)
    }

    pub fn shelf_limit(&self) -> Option<u32> {
        match self.presentation.max_visible_shelves {
            0 => None,
            n => Some(n),
        }
    }
}
