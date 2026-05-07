use core::sync::atomic::Ordering;

pub struct KernelConfig;

impl KernelConfig {
    pub fn arch() -> &'static str {
        TARGET_ARCH
    }

    pub fn rebalance_prefer_local_skip_budget() -> usize {
        MIN_REBALANCE_PREFER_LOCAL_SKIP_BUDGET
    }

    pub fn time_slice() -> u64 {
        10_000_000 // 10ms
    }

    pub fn is_telemetry_enabled() -> bool {
        decode_bool_override(TELEMETRY_ENABLED_OVERRIDE.load(Ordering::Relaxed), true)
    }

    pub fn set_telemetry_enabled(value: Option<bool>) {
        TELEMETRY_ENABLED_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn telemetry_history_len() -> usize {
        load_usize_override_clamped(
            &TELEMETRY_HISTORY_LEN_OVERRIDE,
            DEFAULT_TELEMETRY_HISTORY_LEN,
            1,
            MAX_TELEMETRY_HISTORY_LEN,
        )
    }

    pub fn set_telemetry_history_len(value: Option<usize>) {
        store_usize_override(&TELEMETRY_HISTORY_LEN_OVERRIDE, value);
    }

    pub fn log_level_num() -> u8 {
        load_u8_from_usize_override_clamped(
            &TELEMETRY_LOG_LEVEL_NUM_OVERRIDE,
            DEFAULT_TELEMETRY_LOG_LEVEL_NUM,
            MIN_TELEMETRY_LOG_LEVEL_NUM,
            MAX_TELEMETRY_LOG_LEVEL_NUM,
        )
    }

    pub fn set_log_level_num(value: Option<u8>) {
        store_usize_override(
            &TELEMETRY_LOG_LEVEL_NUM_OVERRIDE,
            value.map(|v| v as usize),
        );
    }

    pub fn is_advanced_debug_enabled() -> bool {
        cfg!(debug_assertions) || cfg!(feature = "debug_test_output")
    }

    pub fn is_virtualization_enabled() -> bool {
        false // Placeholder for future feature
    }

    pub fn is_soft_watchdog_enabled() -> bool {
        SOFT_WATCHDOG_ENABLED_OVERRIDE.load(Ordering::Relaxed) != 0
    }

    pub fn set_soft_watchdog_enabled(value: bool) {
        SOFT_WATCHDOG_ENABLED_OVERRIDE.store(if value { 1 } else { 0 }, Ordering::Relaxed);
    }

    pub fn soft_watchdog_stall_ticks() -> u64 {
        SOFT_WATCHDOG_STALL_TICKS_OVERRIDE.load(Ordering::Relaxed)
    }

    pub fn set_soft_watchdog_stall_ticks(value: u64) {
        SOFT_WATCHDOG_STALL_TICKS_OVERRIDE.store(value, Ordering::Relaxed);
    }

    pub fn soft_watchdog_action_mode() -> WatchdogAction {
        match SOFT_WATCHDOG_ACTION_OVERRIDE.load(Ordering::Relaxed) {
            0 => WatchdogAction::Halt,
            1 => WatchdogAction::Panic,
            _ => WatchdogAction::Log,
        }
    }

    pub fn set_soft_watchdog_action_mode(value: WatchdogAction) {
        SOFT_WATCHDOG_ACTION_OVERRIDE.store(value as u64, Ordering::Relaxed);
    }

    pub fn stack_size() -> usize {
        load_usize_override_clamped(
            &STACK_SIZE_OVERRIDE,
            DEFAULT_STACK_SIZE,
            4096,
            65536,
        )
    }

    pub fn set_stack_size(value: Option<usize>) {
        store_usize_override(&STACK_SIZE_OVERRIDE, value);
    }

    pub fn rebalance_imbalance_threshold() -> usize {
        load_usize_override_clamped(
            &REBALANCE_IMBALANCE_THRESHOLD_OVERRIDE,
            DEFAULT_REBALANCE_IMBALANCE_THRESHOLD,
            1,
            100,
        )
    }

    pub fn set_rebalance_imbalance_threshold(value: Option<usize>) {
        store_usize_override(&REBALANCE_IMBALANCE_THRESHOLD_OVERRIDE, value);
    }

    pub fn should_emit_scheduler_trace_sample(_seq: u64) -> bool {
        SCHEDULER_TRACE_ENABLED_OVERRIDE.load(Ordering::Relaxed) != 0
    }

    pub fn set_scheduler_trace_enabled(value: bool) {
        SCHEDULER_TRACE_ENABLED_OVERRIDE.store(if value { 1 } else { 0 }, Ordering::Relaxed);
    }

    pub fn affinity_policy_mode() -> AffinityPolicy {
        match AFFINITY_POLICY_OVERRIDE.load(Ordering::Relaxed) {
            0 => AffinityPolicy::PreferLocal,
            1 => AffinityPolicy::StrictLocal,
            2 => AffinityPolicy::Balanced,
            _ => AffinityPolicy::Spread,
        }
    }

    pub fn set_affinity_policy_mode(value: AffinityPolicy) {
        AFFINITY_POLICY_OVERRIDE.store(value as u64, Ordering::Relaxed);
    }

    pub fn is_work_stealing_enabled() -> bool {
        WORK_STEALING_ENABLED_OVERRIDE.load(Ordering::Relaxed) != 0
    }

    pub fn set_work_stealing_enabled(value: bool) {
        WORK_STEALING_ENABLED_OVERRIDE.store(if value { 1 } else { 0 }, Ordering::Relaxed);
    }

    pub fn is_periodic_rebalance_enabled() -> bool {
        PERIODIC_REBALANCE_ENABLED_OVERRIDE.load(Ordering::Relaxed) != 0
    }

    pub fn set_periodic_rebalance_enabled(value: bool) {
        PERIODIC_REBALANCE_ENABLED_OVERRIDE.store(if value { 1 } else { 0 }, Ordering::Relaxed);
    }

    pub fn rebalance_interval_ticks() -> u64 {
        REBALANCE_INTERVAL_TICKS_OVERRIDE.load(Ordering::Relaxed)
    }

    pub fn set_rebalance_interval_ticks(value: u64) {
        REBALANCE_INTERVAL_TICKS_OVERRIDE.store(value, Ordering::Relaxed);
    }

    pub fn rebalance_batch_size() -> usize {
        MIN_REBALANCE_BATCH_SIZE
    }

    pub fn is_affinity_enforcement_enabled() -> bool {
        AFFINITY_ENFORCEMENT_ENABLED_OVERRIDE.load(Ordering::Relaxed) != 0
    }

    pub fn set_affinity_enforcement_enabled(value: bool) {
        AFFINITY_ENFORCEMENT_ENABLED_OVERRIDE.store(if value { 1 } else { 0 }, Ordering::Relaxed);
    }

    pub fn affinity_policy() -> AffinityPolicy {
        AffinityPolicy::PreferLocal
    }

    pub fn is_syscall_tracing_enabled() -> bool {
        SYSCALL_TRACING_ENABLED_OVERRIDE.load(Ordering::Relaxed) != 0
    }

    pub fn set_syscall_tracing_enabled(value: bool) {
        SYSCALL_TRACING_ENABLED_OVERRIDE.store(if value { 1 } else { 0 }, Ordering::Relaxed);
    }
}

pub mod control_plane;
pub mod constants;
pub mod debug_macros;
pub mod drivers;
pub mod feature_catalog;
pub mod key_api;
pub mod library_surface;
pub mod network;
pub mod overrides;
pub mod parsers;
pub mod policy;
pub mod policy_profiles;
pub mod profiles;
pub mod reset;
pub mod runtime_key_autogen;
pub mod runtime_tuning;
pub mod scheduler;
pub mod vfs_devfs;
pub mod vfs;
pub mod posix;
pub mod policy_virtualization;

pub use crate::generated_consts::*;
pub use control_plane::*;

pub use profiles::*;
pub use parsers::*;
pub(crate) use constants::*;
pub(crate) use overrides::*;
pub(crate) use debug_macros::*;
pub(crate) use key_api::*;


pub mod legacy_constants;
pub use legacy_constants::*;


