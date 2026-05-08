/// Runtime management interfaces.

use crate::interfaces::KernelResult;
use alloc::string::String;

/// Runtime system state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Initializing,
    Ready,
    Running,
    Paused,
    Error,
    Shutdown,
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub preemption_enabled: bool,
    pub timeslice_ns: u64,
    pub max_tasks: usize,
    pub security_checks_enabled: bool,
    pub telemetry_enabled: bool,
    pub perf_monitoring_enabled: bool,
}

impl RuntimeConfig {
    pub const DEFAULT: Self = Self {
        preemption_enabled: true,
        timeslice_ns: 10_000_000, // 10ms
        max_tasks: 1024,
        security_checks_enabled: true,
        telemetry_enabled: true,
        perf_monitoring_enabled: false,
    };
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Runtime statistics
#[derive(Debug, Clone, Default)]
pub struct RuntimeStats {
    pub tasks_created: u64,
    pub tasks_running: u32,
    pub context_switches: u64,
    pub interrupts_handled: u64,
    pub uptime_us: u64,
    pub boot_complete_time_us: u64,
    pub avg_task_duration_ns: u64,
}

impl RuntimeStats {
    pub const DEFAULT: Self = Self {
        tasks_created: 0,
        tasks_running: 0,
        context_switches: 0,
        interrupts_handled: 0,
        uptime_us: 0,
        boot_complete_time_us: 0,
        avg_task_duration_ns: 0,
    };
}

/// Trait for managing kernel runtime state
pub trait RuntimeManager {
    fn current_state(&self) -> RuntimeState;
    fn set_state(&self, new_state: RuntimeState) -> KernelResult<()>;
    fn config(&self) -> RuntimeConfig;
    fn set_config(&self, config: RuntimeConfig) -> KernelResult<()>;
    fn stats(&self) -> RuntimeStats;
    fn record_context_switch(&self);
    fn record_interrupt(&self, interrupt_id: u32);
    fn check_health(&self) -> bool;
    fn enable_perf_monitoring(&self, enable: bool);
    fn snapshot(&self) -> RuntimeSnapshot;
    fn advance_time(&self, delta_us: u64);
}

/// Runtime snapshot
#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub state: RuntimeState,
    pub config: RuntimeConfig,
    pub stats: RuntimeStats,
    pub timestamp_us: u64,
    pub diagnostics: alloc::vec::Vec<String>,
}
