#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkIoHealthAction {
    NoAction,
    AttemptRebind,
    TriggerFailover,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkIoHealthHarness {
    pub io_error_streak: u64,
    pub rebind_failure_streak: u64,
    pub rebind_threshold: u64,
    pub failover_threshold: u64,
}

impl NetworkIoHealthHarness {
    pub const fn new(rebind_threshold: u64, failover_threshold: u64) -> Self {
        Self {
            io_error_streak: 0,
            rebind_failure_streak: 0,
            rebind_threshold,
            failover_threshold,
        }
    }

    pub fn observe_service_result(&mut self, ok: bool) -> NetworkIoHealthAction {
        if ok {
            self.io_error_streak = 0;
            self.rebind_failure_streak = 0;
            return NetworkIoHealthAction::NoAction;
        }
        self.io_error_streak = self.io_error_streak.saturating_add(1);
        self.recommended_action()
    }

    pub fn observe_rebind_result(&mut self, ok: bool) -> NetworkIoHealthAction {
        self.io_error_streak = 0;
        if ok {
            self.rebind_failure_streak = 0;
            return NetworkIoHealthAction::NoAction;
        }
        self.rebind_failure_streak = self.rebind_failure_streak.saturating_add(1);
        self.recommended_action()
    }

    pub fn recommended_action(&self) -> NetworkIoHealthAction {
        evaluate_network_io_health_action(
            self.io_error_streak,
            self.rebind_failure_streak,
            self.rebind_threshold,
            self.failover_threshold,
        )
    }
}

pub fn evaluate_network_io_health_action(
    io_error_streak: u64,
    rebind_failure_streak: u64,
    rebind_threshold: u64,
    failover_threshold: u64,
) -> NetworkIoHealthAction {
    if failover_threshold > 0 && rebind_failure_streak >= failover_threshold {
        return NetworkIoHealthAction::TriggerFailover;
    }
    if rebind_threshold > 0 && io_error_streak >= rebind_threshold {
        return NetworkIoHealthAction::AttemptRebind;
    }
    NetworkIoHealthAction::NoAction
}
