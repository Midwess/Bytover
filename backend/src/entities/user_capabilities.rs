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
    device_unique_key: Option<String>,
}

impl UserCapabilities {
    pub fn seed(user_order_id: u64, plan: Plan, device_unique_key: Option<String>) -> Self {
        let d = crate::app_gateway::plan::defaults_for(plan);
        Self::from_defaults(user_order_id, plan, d, 0, device_unique_key)
    }

    pub fn from_defaults(
        user_order_id: u64,
        plan: Plan,
        defaults: PlanDefaults,
        bytes_used: u64,
        device_unique_key: Option<String>,
    ) -> Self {
        Self {
            user_order_id,
            plan,
            password_encryption_allowed: defaults.password_encryption_allowed,
            max_files_per_transfer: defaults.max_files_per_transfer,
            total_transfer_bytes_lifetime_cap: defaults.total_transfer_bytes_lifetime_cap,
            total_transfer_bytes_used: bytes_used,
            max_visible_shelves: defaults.max_visible_shelves,
            device_unique_key,
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
        device_unique_key: Option<String>,
    ) -> Self {
        Self {
            user_order_id,
            plan,
            password_encryption_allowed,
            max_files_per_transfer,
            total_transfer_bytes_lifetime_cap,
            total_transfer_bytes_used,
            max_visible_shelves,
            device_unique_key,
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

    pub fn device_unique_key(&self) -> Option<&str> {
        self.device_unique_key.as_deref()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_carries_device_unique_key() {
        let with_key = UserCapabilities::seed(42, Plan::Free, Some("dev-X".to_owned()));
        assert_eq!(with_key.device_unique_key(), Some("dev-X"));

        let without_key = UserCapabilities::seed(43, Plan::Free, None);
        assert_eq!(without_key.device_unique_key(), None);
    }

    #[test]
    fn seed_starts_at_zero_bytes_used() {
        let row = UserCapabilities::seed(42, Plan::Free, Some("dev-X".to_owned()));
        assert_eq!(row.total_transfer_bytes_used(), 0);
    }

    #[test]
    fn from_db_round_trips_device_unique_key() {
        let row = UserCapabilities::from_db(
            7,
            Plan::Free,
            false,
            10,
            8 * 1024 * 1024 * 1024,
            123_456,
            1,
            Some("dev-Y".to_owned()),
        );
        assert_eq!(row.device_unique_key(), Some("dev-Y"));
        assert_eq!(row.total_transfer_bytes_used(), 123_456);
    }

    #[test]
    fn from_db_accepts_null_device_unique_key() {
        let row = UserCapabilities::from_db(7, Plan::Free, false, 10, 0, 0, 1, None);
        assert!(row.device_unique_key().is_none());
    }

    #[test]
    fn paid_seed_with_device_key_records_key_but_unlimited_caps() {
        let row = UserCapabilities::seed(99, Plan::Paid, Some("dev-Z".to_owned()));
        assert_eq!(row.device_unique_key(), Some("dev-Z"));
        assert!(row.password_encryption_allowed());
        assert_eq!(row.total_transfer_bytes_lifetime_cap(), 0);
        assert!(!row.would_exceed_lifetime_cap(u64::MAX));
    }
}
