use crate::entities::user_capabilities::UserCapabilities;
use schema::devlog::app_gateway::models::{
    Capabilities as CapabilitiesMsg, Plan as PlanMsg, PresentationLimits as PresentationLimitsMsg, PricingInfo as PricingInfoMsg,
    TransferLimits as TransferLimitsMsg, TransferUsage as TransferUsageMsg,
};

pub const CAPABILITIES_VERSION: u32 = 1;

pub const FREE_LIFETIME_BYTES_CAP: u64 = 5 * 1024 * 1024 * 1024;
pub const FREE_MAX_FILES_PER_TRANSFER: u32 = 10;
pub const FREE_MAX_VISIBLE_SHELVES: u32 = 1;

pub const PAID_SKU: &str = "bitbridge-onetime-v1";
pub const PAID_PRICE_CURRENCY: &str = "USD";
pub const PAID_PRICE_MINOR_UNITS: u64 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Plan {
    Free,
    Paid,
}

impl Plan {
    pub fn from_i16(value: i16) -> Self {
        match value {
            2 => Plan::Paid,
            _ => Plan::Free,
        }
    }

    pub fn as_i16(self) -> i16 {
        match self {
            Plan::Free => 1,
            Plan::Paid => 2,
        }
    }

    pub fn as_msg(self) -> PlanMsg {
        match self {
            Plan::Free => PlanMsg::Free,
            Plan::Paid => PlanMsg::Paid,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlanDefaults {
    pub password_encryption_allowed: bool,
    pub max_files_per_transfer: u32,
    pub total_transfer_bytes_lifetime_cap: u64,
    pub max_visible_shelves: u32,
}

#[derive(Debug, Clone)]
pub struct PricingInfo {
    pub currency: String,
    pub amount_minor_units: u64,
    pub sku: String,
}

pub fn defaults_for(plan: Plan) -> PlanDefaults {
    match plan {
        Plan::Free => PlanDefaults {
            password_encryption_allowed: false,
            max_files_per_transfer: FREE_MAX_FILES_PER_TRANSFER,
            total_transfer_bytes_lifetime_cap: FREE_LIFETIME_BYTES_CAP,
            max_visible_shelves: FREE_MAX_VISIBLE_SHELVES,
        },
        Plan::Paid => PlanDefaults {
            password_encryption_allowed: true,
            max_files_per_transfer: 0,
            total_transfer_bytes_lifetime_cap: 0,
            max_visible_shelves: 0,
        },
    }
}

pub fn pricing_for(plan: Plan) -> Option<PricingInfo> {
    match plan {
        Plan::Free => Some(PricingInfo {
            currency: PAID_PRICE_CURRENCY.to_owned(),
            amount_minor_units: PAID_PRICE_MINOR_UNITS,
            sku: PAID_SKU.to_owned(),
        }),
        Plan::Paid => None,
    }
}

pub fn build_capabilities_msg(row: &UserCapabilities) -> CapabilitiesMsg {
    let plan = row.plan();
    CapabilitiesMsg {
        plan: plan.as_msg() as i32,
        transfer_limits: TransferLimitsMsg {
            password_encryption_allowed: row.password_encryption_allowed(),
            max_files_per_transfer: row.max_files_per_transfer(),
            total_transfer_bytes_lifetime_cap: row.total_transfer_bytes_lifetime_cap(),
        },
        transfer_usage: TransferUsageMsg {
            total_transfer_bytes_used: row.total_transfer_bytes_used(),
        },
        presentation: PresentationLimitsMsg {
            max_visible_shelves: row.max_visible_shelves(),
        },
        upgrade_pricing: pricing_for(plan).map(|p| PricingInfoMsg {
            currency: p.currency,
            amount_minor_units: p.amount_minor_units,
            sku: p.sku,
        }),
        capabilities_version: CAPABILITIES_VERSION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_defaults() {
        let d = defaults_for(Plan::Free);
        assert!(!d.password_encryption_allowed);
        assert_eq!(d.max_files_per_transfer, 10);
        assert_eq!(d.total_transfer_bytes_lifetime_cap, 5 * 1024 * 1024 * 1024);
        assert_eq!(d.max_visible_shelves, 1);
    }

    #[test]
    fn paid_defaults_unlimited() {
        let d = defaults_for(Plan::Paid);
        assert!(d.password_encryption_allowed);
        assert_eq!(d.max_files_per_transfer, 0);
        assert_eq!(d.total_transfer_bytes_lifetime_cap, 0);
        assert_eq!(d.max_visible_shelves, 0);
    }

    #[test]
    fn pricing_present_for_free_only() {
        assert!(pricing_for(Plan::Free).is_some());
        assert!(pricing_for(Plan::Paid).is_none());
    }
}
