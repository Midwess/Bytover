use crate::app_gateway::plan::{Plan, PlanDefaults};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCapabilities {
    user_order_id: u64,
    plan: Plan,
    password_encryption_allowed: bool,
    max_files_per_transfer: u32,
    total_transfer_bytes_lifetime_cap: u64,
    total_transfer_bytes_used: u64,
    max_visible_shelves: u32,
}

impl UserCapabilities {
    pub fn seed(user_order_id: u64, plan: Plan) -> Self {
        let d = crate::app_gateway::plan::PlanSeeder::defaults_for(plan);
        Self::from_defaults(user_order_id, plan, d, 0)
    }

    pub fn from_defaults(user_order_id: u64, plan: Plan, defaults: PlanDefaults, bytes_used: u64) -> Self {
        Self {
            user_order_id,
            plan,
            password_encryption_allowed: defaults.password_encryption_allowed,
            max_files_per_transfer: defaults.max_files_per_transfer,
            total_transfer_bytes_lifetime_cap: defaults.total_transfer_bytes_lifetime_cap,
            total_transfer_bytes_used: bytes_used,
            max_visible_shelves: defaults.max_visible_shelves,
        }
    }

    pub fn from_db(
        user_order_id: u64,
        plan: Plan,
        password_encryption_allowed: bool,
        max_files_per_transfer: u32,
        total_transfer_bytes_lifetime_cap: u64,
        total_transfer_bytes_used: u64,
        max_visible_shelves: u32,
    ) -> Self {
        Self {
            user_order_id,
            plan,
            password_encryption_allowed,
            max_files_per_transfer,
            total_transfer_bytes_lifetime_cap,
            total_transfer_bytes_used,
            max_visible_shelves,
        }
    }

    pub fn user_order_id(&self) -> u64 {
        self.user_order_id
    }

    pub fn plan(&self) -> Plan {
        self.plan
    }

    pub fn password_encryption_allowed(&self) -> bool {
        self.password_encryption_allowed
    }

    pub fn max_files_per_transfer(&self) -> u32 {
        self.max_files_per_transfer
    }

    pub fn total_transfer_bytes_lifetime_cap(&self) -> u64 {
        self.total_transfer_bytes_lifetime_cap
    }

    pub fn total_transfer_bytes_used(&self) -> u64 {
        self.total_transfer_bytes_used
    }

    pub fn max_visible_shelves(&self) -> u32 {
        self.max_visible_shelves
    }

    pub fn would_exceed_lifetime_cap(&self, delta: u64) -> bool {
        let cap = self.total_transfer_bytes_lifetime_cap;
        if cap == 0 {
            return false;
        }
        self.total_transfer_bytes_used.saturating_add(delta) > cap
    }

    pub fn would_exceed_file_count(&self, total_files: u32) -> Option<u32> {
        let cap = self.max_files_per_transfer;
        if cap != 0 && total_files > cap {
            Some(cap)
        } else {
            None
        }
    }
}
