#[cfg(all(feature = "drivers", feature = "networking"))]
pub(crate) struct NetworkIoHealthDecisionContext {
    pub(crate) io_error_streak: u64,
    pub(crate) rebind_failure_streak: u64,
    pub(crate) driver_failed: bool,
}

#[cfg(all(feature = "drivers", feature = "networking"))]
pub(crate) fn decide_network_io_health_action(
    context: NetworkIoHealthDecisionContext,
) -> aethercore::modules::drivers::NetworkIoHealthAction {
    use crate::kernel_runtime::networking::{NETWORK_IO_REBIND_STREAK_THRESHOLD, NETWORK_IO_FAILOVER_STREAK_THRESHOLD};
    let action = aethercore::modules::drivers::evaluate_network_io_health_action(
        context.io_error_streak,
        context.rebind_failure_streak,
        NETWORK_IO_REBIND_STREAK_THRESHOLD,
        NETWORK_IO_FAILOVER_STREAK_THRESHOLD,
    );
    if context.driver_failed
        && matches!(
            action,
            aethercore::modules::drivers::NetworkIoHealthAction::NoAction
        )
    {
        return if context.rebind_failure_streak
            >= NETWORK_IO_FAILOVER_STREAK_THRESHOLD
        {
            aethercore::modules::drivers::NetworkIoHealthAction::TriggerFailover
        } else {
            aethercore::modules::drivers::NetworkIoHealthAction::AttemptRebind
        };
    }
    action
}

#[cfg(all(test, feature = "drivers", feature = "networking"))]
mod tests {
    use super::*;
    use crate::kernel_runtime::networking::{
        NETWORK_IO_FAILOVER_STREAK_THRESHOLD,
        NETWORK_IO_REBIND_STREAK_THRESHOLD,
    };

    #[test_case]
    fn io_health_decision_escalates_from_rebind_to_failover() {
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: 1,
                rebind_failure_streak: 0,
                driver_failed: false,
            }),
            aethercore::modules::drivers::NetworkIoHealthAction::NoAction
        ));
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: NETWORK_IO_REBIND_STREAK_THRESHOLD,
                rebind_failure_streak: 0,
                driver_failed: false,
            }),
            aethercore::modules::drivers::NetworkIoHealthAction::AttemptRebind
        ));
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: 0,
                rebind_failure_streak: NETWORK_IO_FAILOVER_STREAK_THRESHOLD,
                driver_failed: false,
            }),
            aethercore::modules::drivers::NetworkIoHealthAction::TriggerFailover
        ));
    }

    #[test_case]
    fn io_health_decision_failed_driver_forces_rebind() {
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: 1,
                rebind_failure_streak: 0,
                driver_failed: true,
            }),
            aethercore::modules::drivers::NetworkIoHealthAction::AttemptRebind
        ));
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: 0,
                rebind_failure_streak: NETWORK_IO_FAILOVER_STREAK_THRESHOLD,
                driver_failed: true,
            }),
            aethercore::modules::drivers::NetworkIoHealthAction::TriggerFailover
        ));
    }
}
