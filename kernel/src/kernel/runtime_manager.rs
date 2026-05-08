// --- PHASE 4: RUNTIME MANAGER IMPLEMENTATION ---
// Concrete runtime state machine and telemetry collection

use crate::core::log;
use alloc::format;
use crate::interfaces::runtime::{
    RuntimeConfig, RuntimeManager, RuntimeSnapshot, RuntimeState, RuntimeStats,
};
use crate::kernel::sync::IrqSafeMutex;
use core::sync::atomic::{AtomicU64, Ordering};

/// Concrete implementation of RuntimeManager
pub struct ConcreteRuntimeManager {
    /// Current runtime state
    state: IrqSafeMutex<RuntimeState>,

    /// Runtime configuration
    config: IrqSafeMutex<RuntimeConfig>,

    /// Runtime statistics
    stats: IrqSafeMutex<RuntimeStats>,

    /// Monotonic timestamp counter
    timestamp_ns: AtomicU64,
}

impl ConcreteRuntimeManager {
    /// Create a new runtime manager
    pub const fn new() -> Self {
        Self {
            state: IrqSafeMutex::new(RuntimeState::Initializing),
            config: IrqSafeMutex::new(RuntimeConfig::DEFAULT),
            stats: IrqSafeMutex::new(RuntimeStats::DEFAULT),
            timestamp_ns: AtomicU64::new(0),
        }
    }

    /// Initialize the runtime manager at boot
    pub fn initialize() -> crate::interfaces::KernelResult<&'static ConcreteRuntimeManager> {
        log::info("Initializing RuntimeManager");

        // Verify configuration consistency
        let cfg = GLOBAL_RUNTIME_MANAGER.config.lock();
        if cfg.preemption_enabled && cfg.timeslice_ns == 0 {
            return Err(crate::interfaces::KernelError::InvalidInput);
        }

        Ok(&GLOBAL_RUNTIME_MANAGER)
    }
}

// Global runtime manager instance
pub static GLOBAL_RUNTIME_MANAGER: ConcreteRuntimeManager = ConcreteRuntimeManager::new();

impl RuntimeManager for ConcreteRuntimeManager {
    /// Get current runtime state
    fn current_state(&self) -> RuntimeState {
        *self.state.lock()
    }

    /// Set runtime state (with validation)
    fn set_state(&self, new_state: RuntimeState) -> crate::interfaces::KernelResult<()> {
        let current = *self.state.lock();

        // Validate state transitions
        let valid = match (current, new_state) {
            (RuntimeState::Initializing, RuntimeState::Ready) => true,
            (RuntimeState::Ready, RuntimeState::Running) => true,
            (RuntimeState::Running, RuntimeState::Paused) => true,
            (RuntimeState::Running, RuntimeState::Error) => true,
            (RuntimeState::Paused, RuntimeState::Running) => true,
            (RuntimeState::Paused, RuntimeState::Error) => true,
            (_, RuntimeState::Shutdown) => true, // Can shutdown from any state
            _ => false,
        };

        if !valid {
            log::warn(&format!(
                "Invalid state transition: {:?} -> {:?}",
                current, new_state
            ));
            return Err(crate::interfaces::KernelError::InvalidInput);
        }

        *self.state.lock() = new_state;
        log::info(&format!("Runtime state changed to: {:?}", new_state));

        // Update telemetry
        if new_state == RuntimeState::Running {
            let mut stats = self.stats.lock();
            stats.boot_complete_time_us = self.timestamp_ns.load(Ordering::Relaxed) / 1000;
        }

        Ok(())
    }

    /// Get runtime configuration
    fn config(&self) -> RuntimeConfig {
        self.config.lock().clone()
    }

    /// Update runtime configuration
    fn set_config(&self, config: RuntimeConfig) -> crate::interfaces::KernelResult<()> {
        // Validate new configuration
        if config.preemption_enabled && config.timeslice_ns == 0 {
            return Err(crate::interfaces::KernelError::InvalidInput);
        }

        if config.max_tasks == 0 {
            return Err(crate::interfaces::KernelError::InvalidInput);
        }

        *self.config.lock() = config;
        log::debug("Runtime configuration updated");
        Ok(())
    }

    /// Get current runtime statistics
    fn stats(&self) -> RuntimeStats {
        self.stats.lock().clone()
    }

    /// Increment context switch counter
    fn record_context_switch(&self) {
        self.stats.lock().context_switches += 1;
    }

    /// Increment interrupt counter
    fn record_interrupt(&self, _interrupt_id: u32) {
        self.stats.lock().interrupts_handled += 1;
    }

    /// Check runtime health
    fn check_health(&self) -> bool {
        let state = *self.state.lock();
        let stats = self.stats.lock();

        // Sanity checks
        match state {
            RuntimeState::Initializing => true,
            RuntimeState::Ready => true,
            RuntimeState::Running => {
                // Check for stuck processes (no context switches)
                stats.context_switches > 0
            }
            RuntimeState::Paused => true,
            RuntimeState::Error => false,
            RuntimeState::Shutdown => true,
        }
    }

    /// Enable performance monitoring
    fn enable_perf_monitoring(&self, _enable: bool) {
        log::debug("Performance monitoring control called");
    }

    /// Get runtime snapshot for debugging
    fn snapshot(&self) -> RuntimeSnapshot {
        let state = *self.state.lock();
        let config = self.config.lock().clone();
        let stats = self.stats.lock().clone();

        RuntimeSnapshot {
            state,
            config,
            stats,
            timestamp_us: self.timestamp_ns.load(Ordering::Relaxed) / 1000,
            diagnostics: alloc::vec::Vec::new(),
        }
    }

    /// Update system time (called by timer interrupt)
    fn advance_time(&self, delta_us: u64) {
        let current_ns = delta_us * 1000;
        self.timestamp_ns
            .fetch_add(current_ns, Ordering::Relaxed);

        // Update uptime
        let mut stats = self.stats.lock();
        stats.uptime_us += delta_us;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_manager_creation() {
        let mgr = ConcreteRuntimeManager::new();
        assert_eq!(mgr.current_state(), RuntimeState::Initializing);
    }

    #[test]
    fn test_config_default() {
        let config = RuntimeConfig::default();
        assert!(config.preemption_enabled);
        assert_eq!(config.max_tasks, 1024);
    }

    #[test]
    fn test_config_validation() {
        let mgr = ConcreteRuntimeManager::new();

        // Invalid: preemption enabled but no timeslice
        let bad_config = RuntimeConfig {
            preemption_enabled: true,
            timeslice_ns: 0,
            max_tasks: 1024,
            security_checks_enabled: true,
            telemetry_enabled: true,
            perf_monitoring_enabled: false,
        };
        assert!(mgr.set_config(bad_config).is_err());

        // Invalid: zero max_tasks
        let bad_config = RuntimeConfig {
            preemption_enabled: true,
            timeslice_ns: 10_000_000,
            max_tasks: 0,
            security_checks_enabled: true,
            telemetry_enabled: true,
            perf_monitoring_enabled: false,
        };
        assert!(mgr.set_config(bad_config).is_err());
    }

    #[test]
    fn test_stats_context_switch() {
        let mgr = ConcreteRuntimeManager::new();
        let initial = mgr.stats().context_switches;

        mgr.record_context_switch();
        let after = mgr.stats().context_switches;

        assert_eq!(after, initial + 1);
    }

    #[test]
    fn test_health_check_running() {
        let mgr = ConcreteRuntimeManager::new();

        mgr.set_state(RuntimeState::Ready).ok();
        mgr.set_state(RuntimeState::Running).ok();

        // Unhealthy: running but no context switches
        assert!(!mgr.check_health());

        // Healthy: running with context switches
        mgr.record_context_switch();
        assert!(mgr.check_health());
    }

    #[test]
    fn test_time_advancement() {
        let mgr = ConcreteRuntimeManager::new();
        let initial = mgr.stats().uptime_us;

        mgr.advance_time(1000); // 1000 microseconds
        let after = mgr.stats().uptime_us;

        assert_eq!(after, initial + 1000);
    }

    #[test]
    fn test_snapshot() {
        let mgr = ConcreteRuntimeManager::new();
        mgr.set_state(RuntimeState::Ready).ok();

        let snap = mgr.snapshot();
        assert_eq!(snap.state, RuntimeState::Ready);
        assert!(snap.config.preemption_enabled);
    }
}
