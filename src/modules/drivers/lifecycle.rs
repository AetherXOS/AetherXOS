use crate::hal::pci::PciDevice;

const DEFAULT_RECOVERY_MAX_ATTEMPTS: u8 = 3;
const DEFAULT_RECOVERY_COOLDOWN_TICKS: u8 = 2;
const DEFAULT_DEGRADE_THRESHOLD: u8 = 1;
const RECOVERY_FAIL_THRESHOLD: u8 = 5;
const RECOVERY_START_THRESHOLD: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverClass {
    Network,
    Storage,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverHealth {
    Healthy,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverState {
    Unknown,
    Discovered,
    Initializing,
    Ready,
    Degraded,
    Recovering,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverErrorKind {
    Probe,
    Init,
    Io,
    Teardown,
    Timeout,
    Unsupported,
    InvalidConfig,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverIoGate {
    Open,
    Cooldown,
    Closed,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverRecoveryPolicy {
    pub max_recovery_attempts: u8,
    pub cooldown_ticks: u8,
    pub degrade_after_consecutive_io_failures: u8,
    pub recover_after_consecutive_io_failures: u8,
    pub fail_after_consecutive_io_failures: u8,
}

impl DriverRecoveryPolicy {
    pub const fn default() -> Self {
        Self {
            max_recovery_attempts: DEFAULT_RECOVERY_MAX_ATTEMPTS,
            cooldown_ticks: DEFAULT_RECOVERY_COOLDOWN_TICKS,
            degrade_after_consecutive_io_failures: DEFAULT_DEGRADE_THRESHOLD,
            recover_after_consecutive_io_failures: RECOVERY_START_THRESHOLD,
            fail_after_consecutive_io_failures: RECOVERY_FAIL_THRESHOLD,
        }
    }

    pub const fn sanitized(self) -> Self {
        let degrade = if self.degrade_after_consecutive_io_failures == 0 {
            1
        } else {
            self.degrade_after_consecutive_io_failures
        };
        let recover = if self.recover_after_consecutive_io_failures < degrade {
            degrade
        } else {
            self.recover_after_consecutive_io_failures
        };
        let fail = if self.fail_after_consecutive_io_failures < recover {
            recover
        } else {
            self.fail_after_consecutive_io_failures
        };

        Self {
            max_recovery_attempts: self.max_recovery_attempts,
            cooldown_ticks: self.cooldown_ticks,
            degrade_after_consecutive_io_failures: degrade,
            recover_after_consecutive_io_failures: recover,
            fail_after_consecutive_io_failures: fail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverStatus {
    pub state: DriverState,
    pub health: DriverHealth,
    pub last_error: Option<DriverErrorKind>,
    pub fault_count: u32,
    pub io_success_count: u32,
    pub io_failure_count: u32,
    pub transition_count: u32,
    pub recovery_budget_total: u8,
    pub recovery_budget_used: u8,
    pub recovery_budget_remaining: u8,
    pub recovery_cooldown_ticks_remaining: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverStateMachine {
    state: DriverState,
    last_error: Option<DriverErrorKind>,
    fault_count: u32,
    io_success_count: u32,
    io_failure_count: u32,
    transition_count: u32,
    consecutive_io_failures: u8,
    recovery_policy: DriverRecoveryPolicy,
    recovery_budget_used: u8,
    recovery_cooldown_ticks_remaining: u8,
}

impl DriverStateMachine {
    pub const fn new_discovered() -> Self {
        Self::new_discovered_with_policy(DriverRecoveryPolicy::default())
    }

    pub const fn new_discovered_with_policy(recovery_policy: DriverRecoveryPolicy) -> Self {
        Self {
            state: DriverState::Discovered,
            last_error: None,
            fault_count: 0,
            io_success_count: 0,
            io_failure_count: 0,
            transition_count: 0,
            consecutive_io_failures: 0,
            recovery_policy: recovery_policy.sanitized(),
            recovery_budget_used: 0,
            recovery_cooldown_ticks_remaining: 0,
        }
    }

    #[inline(always)]
    fn set_state(&mut self, state: DriverState) {
        if self.state != state {
            self.state = state;
            self.transition_count = self.transition_count.saturating_add(1);
        }
    }

    pub fn on_init_start(&mut self) {
        self.set_state(DriverState::Initializing);
    }

    pub fn on_init_success(&mut self) {
        self.set_state(DriverState::Ready);
        self.last_error = None;
        self.consecutive_io_failures = 0;
        self.recovery_budget_used = 0;
        self.recovery_cooldown_ticks_remaining = 0;
    }

    pub fn on_init_failure(&mut self, kind: DriverErrorKind) {
        self.set_state(DriverState::Failed);
        self.last_error = Some(kind);
        self.fault_count = self.fault_count.saturating_add(1);
    }

    pub fn on_io_success(&mut self) {
        self.io_success_count = self.io_success_count.saturating_add(1);
        self.consecutive_io_failures = 0;
        self.recovery_cooldown_ticks_remaining = 0;
        if matches!(self.state, DriverState::Degraded | DriverState::Recovering) {
            self.set_state(DriverState::Ready);
        }
    }

    pub fn on_io_failure(&mut self, kind: DriverErrorKind) {
        self.last_error = Some(kind);
        self.fault_count = self.fault_count.saturating_add(1);
        self.io_failure_count = self.io_failure_count.saturating_add(1);
        self.consecutive_io_failures = self.consecutive_io_failures.saturating_add(1);

        if self.consecutive_io_failures >= self.recovery_policy.fail_after_consecutive_io_failures {
            self.set_state(DriverState::Failed);
        } else if self.consecutive_io_failures
            >= self.recovery_policy.recover_after_consecutive_io_failures
        {
            if self.recovery_budget_used >= self.recovery_policy.max_recovery_attempts {
                self.set_state(DriverState::Failed);
            } else {
                self.recovery_budget_used = self.recovery_budget_used.saturating_add(1);
                self.recovery_cooldown_ticks_remaining = self.recovery_policy.cooldown_ticks;
                self.set_state(DriverState::Recovering);
            }
        } else {
            if self.consecutive_io_failures
                >= self.recovery_policy.degrade_after_consecutive_io_failures
            {
                self.set_state(DriverState::Degraded);
            }
        }
    }

    pub fn on_recover_attempt(&mut self) {
        if self.state == DriverState::Degraded {
            if self.recovery_budget_used >= self.recovery_policy.max_recovery_attempts {
                self.set_state(DriverState::Failed);
                return;
            }
            self.recovery_budget_used = self.recovery_budget_used.saturating_add(1);
            self.recovery_cooldown_ticks_remaining = self.recovery_policy.cooldown_ticks;
            self.set_state(DriverState::Recovering);
        }
    }

    pub fn on_recover_success(&mut self) {
        self.consecutive_io_failures = 0;
        self.last_error = None;
        self.recovery_cooldown_ticks_remaining = 0;
        self.set_state(DriverState::Ready);
    }

    pub fn io_gate(&mut self) -> DriverIoGate {
        if matches!(self.state, DriverState::Failed | DriverState::Stopped) {
            return DriverIoGate::Closed;
        }

        if self.state == DriverState::Recovering && self.recovery_cooldown_ticks_remaining > 0 {
            self.recovery_cooldown_ticks_remaining =
                self.recovery_cooldown_ticks_remaining.saturating_sub(1);
            return DriverIoGate::Cooldown;
        }

        DriverIoGate::Open
    }

    pub fn should_service_io(&mut self) -> bool {
        self.io_gate() == DriverIoGate::Open
    }

    pub fn on_teardown(&mut self) {
        self.set_state(DriverState::Stopped);
    }

    pub fn status(&self) -> DriverStatus {
        let remaining = self
            .recovery_policy
            .max_recovery_attempts
            .saturating_sub(self.recovery_budget_used);

        DriverStatus {
            state: self.state,
            health: map_state_to_health(self.state),
            last_error: self.last_error,
            fault_count: self.fault_count,
            io_success_count: self.io_success_count,
            io_failure_count: self.io_failure_count,
            transition_count: self.transition_count,
            recovery_budget_total: self.recovery_policy.max_recovery_attempts,
            recovery_budget_used: self.recovery_budget_used,
            recovery_budget_remaining: remaining,
            recovery_cooldown_ticks_remaining: self.recovery_cooldown_ticks_remaining,
        }
    }

    pub fn health(&self) -> DriverHealth {
        map_state_to_health(self.state)
    }
}

#[inline(always)]
fn map_state_to_health(state: DriverState) -> DriverHealth {
    match state {
        DriverState::Ready => DriverHealth::Healthy,
        DriverState::Failed => DriverHealth::Failed,
        _ => DriverHealth::Degraded,
    }
}

pub trait DriverLifecycle {
    fn class(&self) -> DriverClass;
    fn name(&self) -> &'static str;
    fn init_driver(&mut self) -> Result<(), &'static str>;
    fn service_io(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    fn teardown(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    fn health(&self) -> DriverHealth;
    fn status(&self) -> DriverStatus;
}

pub trait LifecycleAdapter {
    fn driver_class(&self) -> DriverClass;
    fn driver_name(&self) -> &'static str;
    fn lifecycle_state(&self) -> &DriverStateMachine;
    fn init_adapter(&mut self) -> Result<(), &'static str>;
    fn service_adapter(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    fn teardown_adapter(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<T: LifecycleAdapter> DriverLifecycle for T {
    fn class(&self) -> DriverClass {
        self.driver_class()
    }

    fn name(&self) -> &'static str {
        self.driver_name()
    }

    fn init_driver(&mut self) -> Result<(), &'static str> {
        self.init_adapter()
    }

    fn service_io(&mut self) -> Result<(), &'static str> {
        self.service_adapter()
    }

    fn teardown(&mut self) -> Result<(), &'static str> {
        self.teardown_adapter()
    }

    fn health(&self) -> DriverHealth {
        self.lifecycle_state().health()
    }

    fn status(&self) -> DriverStatus {
        self.lifecycle_state().status()
    }
}

#[macro_export]
macro_rules! impl_lifecycle_adapter {
    (
        for $ty:ty,
        class: $class:expr,
        name: $name:expr,
        lifecycle: $lifecycle:ident,
        init: $init:ident,
        service: $service:ident,
        teardown: $teardown:ident $(,)?
    ) => {
        impl $crate::modules::drivers::lifecycle::LifecycleAdapter for $ty {
            fn driver_class(&self) -> $crate::modules::drivers::lifecycle::DriverClass {
                $class
            }

            fn driver_name(&self) -> &'static str {
                $name
            }

            fn lifecycle_state(&self) -> &$crate::modules::drivers::lifecycle::DriverStateMachine {
                &self.$lifecycle
            }

            fn init_adapter(&mut self) -> Result<(), &'static str> {
                self.$init()
            }

            fn service_adapter(&mut self) -> Result<(), &'static str> {
                self.$service()
            }

            fn teardown_adapter(&mut self) -> Result<(), &'static str> {
                self.$teardown()
            }
        }
    };
}

pub trait PciProbeDriver: Sized {
    fn probe_pci(devices: &[PciDevice]) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn state_machine_transitions_through_standard_lifecycle() {
        let mut sm = DriverStateMachine::new_discovered();
        assert_eq!(sm.status().state, DriverState::Discovered);

        sm.on_init_start();
        assert_eq!(sm.status().state, DriverState::Initializing);

        sm.on_init_success();
        assert_eq!(sm.status().health, DriverHealth::Healthy);

        sm.on_io_failure(DriverErrorKind::Io);
        assert_eq!(sm.status().state, DriverState::Degraded);

        sm.on_io_failure(DriverErrorKind::Io);
        sm.on_io_failure(DriverErrorKind::Io);
        assert_eq!(sm.status().state, DriverState::Recovering);
        assert_eq!(sm.status().recovery_budget_used, 1);

        sm.on_io_failure(DriverErrorKind::Io);
        sm.on_io_failure(DriverErrorKind::Io);
        assert_eq!(sm.status().state, DriverState::Failed);

        sm.on_teardown();
        assert_eq!(sm.status().state, DriverState::Stopped);
        assert!(!sm.should_service_io());
    }

    #[test_case]
    fn io_recovery_path_returns_to_ready() {
        let mut sm = DriverStateMachine::new_discovered_with_policy(DriverRecoveryPolicy {
            max_recovery_attempts: 3,
            cooldown_ticks: 1,
            ..DriverRecoveryPolicy::default()
        });
        sm.on_init_start();
        sm.on_init_success();
        sm.on_io_failure(DriverErrorKind::Io);
        sm.on_recover_attempt();
        assert_eq!(sm.io_gate(), DriverIoGate::Cooldown);
        assert_eq!(sm.io_gate(), DriverIoGate::Open);
        sm.on_io_success();
        sm.on_recover_success();

        let st = sm.status();
        assert_eq!(st.state, DriverState::Ready);
        assert_eq!(st.health, DriverHealth::Healthy);
        assert!(st.io_success_count >= 1);
    }

    #[test_case]
    fn recovery_budget_exhaustion_transitions_to_failed() {
        let mut sm = DriverStateMachine::new_discovered_with_policy(DriverRecoveryPolicy {
            max_recovery_attempts: 1,
            cooldown_ticks: 0,
            ..DriverRecoveryPolicy::default()
        });
        sm.on_init_start();
        sm.on_init_success();

        sm.on_io_failure(DriverErrorKind::Io);
        sm.on_io_failure(DriverErrorKind::Io);
        sm.on_io_failure(DriverErrorKind::Io);
        assert_eq!(sm.status().state, DriverState::Recovering);
        assert_eq!(sm.status().recovery_budget_used, 1);

        sm.on_io_success();
        sm.on_io_failure(DriverErrorKind::Io);
        sm.on_io_failure(DriverErrorKind::Io);
        sm.on_io_failure(DriverErrorKind::Io);
        assert_eq!(sm.status().state, DriverState::Failed);
        assert_eq!(sm.io_gate(), DriverIoGate::Closed);
    }
}
