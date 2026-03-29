#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) struct NetworkIoHealthDecisionContext {
    pub(super) io_error_streak: u64,
    pub(super) rebind_failure_streak: u64,
    pub(super) driver_failed: bool,
}

#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) fn decide_network_io_health_action(
    context: NetworkIoHealthDecisionContext,
) -> hypercore::modules::drivers::NetworkIoHealthAction {
    let action = hypercore::modules::drivers::evaluate_network_io_health_action(
        context.io_error_streak,
        context.rebind_failure_streak,
        super::super::NETWORK_IO_REBIND_STREAK_THRESHOLD,
        super::super::NETWORK_IO_FAILOVER_STREAK_THRESHOLD,
    );
    if context.driver_failed
        && matches!(
            action,
            hypercore::modules::drivers::NetworkIoHealthAction::NoAction
        )
    {
        return if context.rebind_failure_streak
            >= super::super::NETWORK_IO_FAILOVER_STREAK_THRESHOLD
        {
            hypercore::modules::drivers::NetworkIoHealthAction::TriggerFailover
        } else {
            hypercore::modules::drivers::NetworkIoHealthAction::AttemptRebind
        };
    }
    action
}

#[cfg(all(test, feature = "drivers", feature = "networking"))]
mod tests {
    use super::*;

    #[test_case]
    fn io_health_decision_escalates_from_rebind_to_failover() {
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: 1,
                rebind_failure_streak: 0,
                driver_failed: false,
            }),
            hypercore::modules::drivers::NetworkIoHealthAction::NoAction
        ));
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: super::super::super::NETWORK_IO_REBIND_STREAK_THRESHOLD,
                rebind_failure_streak: 0,
                driver_failed: false,
            }),
            hypercore::modules::drivers::NetworkIoHealthAction::AttemptRebind
        ));
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: 0,
                rebind_failure_streak: super::super::super::NETWORK_IO_FAILOVER_STREAK_THRESHOLD,
                driver_failed: false,
            }),
            hypercore::modules::drivers::NetworkIoHealthAction::TriggerFailover
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
            hypercore::modules::drivers::NetworkIoHealthAction::AttemptRebind
        ));
        assert!(matches!(
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: 0,
                rebind_failure_streak: super::super::super::NETWORK_IO_FAILOVER_STREAK_THRESHOLD,
                driver_failed: true,
            }),
            hypercore::modules::drivers::NetworkIoHealthAction::TriggerFailover
        ));
    }
}
