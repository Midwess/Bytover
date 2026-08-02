#[cfg(feature = "mac-app-store")]
pub(crate) const DISTRIBUTION_MARKER: &str = "BYTOVER_DISTRIBUTION_CHANNEL=mac-app-store";
#[cfg(not(feature = "mac-app-store"))]
pub(crate) const DISTRIBUTION_MARKER: &str = "BYTOVER_DISTRIBUTION_CHANNEL=direct";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum DesktopDistribution {
    Direct,
    MacAppStore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StartupPolicy {
    allows_privileged_startup: bool,
}

impl StartupPolicy {
    pub(crate) const fn requests_privileged_permissions(self) -> bool {
        self.allows_privileged_startup
    }

    pub(crate) const fn opens_permission_settings(self) -> bool {
        self.allows_privileged_startup
    }

    pub(crate) const fn starts_global_input_monitor(self) -> bool {
        self.allows_privileged_startup
    }

    pub(crate) const fn starts_drag_pasteboard_monitor(self) -> bool {
        self.allows_privileged_startup
    }
}

pub(crate) const fn startup_policy(distribution: DesktopDistribution) -> StartupPolicy {
    StartupPolicy {
        allows_privileged_startup: matches!(distribution, DesktopDistribution::Direct),
    }
}

pub(crate) const fn current_startup_policy() -> StartupPolicy {
    #[cfg(feature = "mac-app-store")]
    return startup_policy(DesktopDistribution::MacAppStore);

    #[cfg(not(feature = "mac-app-store"))]
    startup_policy(DesktopDistribution::Direct)
}

#[cfg(test)]
mod tests {
    use super::{
        current_startup_policy, startup_policy, DesktopDistribution, DISTRIBUTION_MARKER,
    };

    #[test]
    fn app_store_policy_disables_privileged_startup_behavior() {
        let policy = startup_policy(DesktopDistribution::MacAppStore);

        assert!(!policy.requests_privileged_permissions());
        assert!(!policy.opens_permission_settings());
        assert!(!policy.starts_global_input_monitor());
        assert!(!policy.starts_drag_pasteboard_monitor());
    }

    #[test]
    fn direct_distribution_policy_preserves_existing_startup_behavior() {
        let policy = startup_policy(DesktopDistribution::Direct);

        assert!(policy.requests_privileged_permissions());
        assert!(policy.opens_permission_settings());
        assert!(policy.starts_global_input_monitor());
        assert!(policy.starts_drag_pasteboard_monitor());
    }

    #[test]
    fn compiled_distribution_uses_the_matching_policy_and_marker() {
        let policy = current_startup_policy();

        #[cfg(feature = "mac-app-store")]
        {
            assert_eq!(
                policy,
                startup_policy(DesktopDistribution::MacAppStore)
            );
            assert_eq!(
                DISTRIBUTION_MARKER,
                "BYTOVER_DISTRIBUTION_CHANNEL=mac-app-store"
            );
        }

        #[cfg(not(feature = "mac-app-store"))]
        {
            assert_eq!(policy, startup_policy(DesktopDistribution::Direct));
            assert_eq!(
                DISTRIBUTION_MARKER,
                "BYTOVER_DISTRIBUTION_CHANNEL=direct"
            );
        }
    }
}
