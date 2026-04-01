use core::sync::atomic::Ordering;

use super::*;

impl KernelConfig {
    pub fn irq_vector_base() -> u8 {
        let override_value = IRQ_VECTOR_BASE_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_IRQ_VECTOR_BASE
        } else {
            u8::try_from(override_value)
                .unwrap_or(DEFAULT_IRQ_VECTOR_BASE)
                .clamp(DEFAULT_IRQ_VECTOR_BASE, MAX_IRQ_VECTOR_BASE)
        }
    }

    pub fn set_irq_vector_base(value: Option<u8>) {
        IRQ_VECTOR_BASE_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn watchdog_hard_stall_ns() -> u64 {
        load_u64_override_clamped(
            &WATCHDOG_HARD_STALL_NS_OVERRIDE,
            DEFAULT_WATCHDOG_HARD_STALL_NS,
            1,
            MAX_WATCHDOG_HARD_STALL_NS,
        )
    }

    pub fn set_watchdog_hard_stall_ns(value: Option<u64>) {
        store_u64_override(&WATCHDOG_HARD_STALL_NS_OVERRIDE, value);
    }

    pub fn rt_force_reschedule_min_ticks() -> usize {
        load_usize_override_clamped(
            &RT_FORCE_RESCHEDULE_MIN_TICKS_OVERRIDE,
            DEFAULT_RT_FORCE_MIN_TICKS,
            DEFAULT_RT_FORCE_MIN_TICKS,
            MAX_RT_FORCE_MIN_TICKS,
        )
    }

    pub fn set_rt_force_reschedule_min_ticks(value: Option<usize>) {
        store_usize_override(&RT_FORCE_RESCHEDULE_MIN_TICKS_OVERRIDE, value);
    }

    pub fn rt_deadline_burst_threshold() -> usize {
        load_usize_override_clamped(
            &RT_DEADLINE_BURST_THRESHOLD_OVERRIDE,
            DEFAULT_RT_DEADLINE_BURST_THRESHOLD,
            DEFAULT_RT_FORCE_MIN_TICKS,
            MAX_RT_DEADLINE_BURST_THRESHOLD,
        )
    }

    pub fn set_rt_deadline_burst_threshold(value: Option<usize>) {
        store_usize_override(&RT_DEADLINE_BURST_THRESHOLD_OVERRIDE, value);
    }

    pub fn module_loader_max_load_segments() -> usize {
        load_usize_override_clamped(
            &MODULE_LOADER_MAX_LOAD_SEGMENTS_OVERRIDE,
            DEFAULT_MODULE_LOADER_MAX_LOAD_SEGMENTS,
            1,
            MAX_MODULE_LOADER_MAX_LOAD_SEGMENTS,
        )
    }

    pub fn set_module_loader_max_load_segments(value: Option<usize>) {
        store_usize_override(&MODULE_LOADER_MAX_LOAD_SEGMENTS_OVERRIDE, value);
    }

    pub fn module_loader_max_total_image_bytes() -> u64 {
        load_u64_override_clamped(
            &MODULE_LOADER_MAX_TOTAL_IMAGE_BYTES_OVERRIDE,
            DEFAULT_MODULE_LOADER_MAX_TOTAL_IMAGE_BYTES,
            1,
            MAX_MODULE_LOADER_MAX_TOTAL_IMAGE_BYTES,
        )
    }

    pub fn set_module_loader_max_total_image_bytes(value: Option<u64>) {
        store_u64_override(&MODULE_LOADER_MAX_TOTAL_IMAGE_BYTES_OVERRIDE, value);
    }

    pub fn launch_max_process_name_len() -> usize {
        load_usize_override_clamped(
            &LAUNCH_MAX_PROCESS_NAME_LEN_OVERRIDE,
            DEFAULT_LAUNCH_MAX_PROCESS_NAME_LEN,
            1,
            MAX_LAUNCH_MAX_PROCESS_NAME_LEN,
        )
    }

    pub fn set_launch_max_process_name_len(value: Option<usize>) {
        store_usize_override(&LAUNCH_MAX_PROCESS_NAME_LEN_OVERRIDE, value);
    }

    pub fn launch_max_boot_image_bytes() -> usize {
        load_usize_override_clamped(
            &LAUNCH_MAX_BOOT_IMAGE_BYTES_OVERRIDE,
            DEFAULT_LAUNCH_MAX_BOOT_IMAGE_BYTES,
            1,
            MAX_LAUNCH_MAX_BOOT_IMAGE_BYTES,
        )
    }

    pub fn set_launch_max_boot_image_bytes(value: Option<usize>) {
        store_usize_override(&LAUNCH_MAX_BOOT_IMAGE_BYTES_OVERRIDE, value);
    }

    pub fn launch_handoff_stage_timeout_epochs() -> u64 {
        load_u64_override_clamped(
            &LAUNCH_STAGE_TIMEOUT_EPOCHS_OVERRIDE,
            DEFAULT_LAUNCH_STAGE_TIMEOUT_EPOCHS,
            1,
            MAX_LAUNCH_STAGE_TIMEOUT_EPOCHS,
        )
    }

    pub fn set_launch_handoff_stage_timeout_epochs(value: Option<u64>) {
        store_u64_override(&LAUNCH_STAGE_TIMEOUT_EPOCHS_OVERRIDE, value);
    }

    pub fn power_runqueue_saturation_limit() -> usize {
        load_usize_override_clamped(
            &POWER_RUNQUEUE_SATURATION_LIMIT_OVERRIDE,
            DEFAULT_POWER_RUNQUEUE_SATURATION_LIMIT,
            1,
            MAX_POWER_RUNQUEUE_SATURATION_LIMIT,
        )
    }

    pub fn set_power_runqueue_saturation_limit(value: Option<usize>) {
        store_usize_override(&POWER_RUNQUEUE_SATURATION_LIMIT_OVERRIDE, value);
    }

    pub fn irqsafe_mutex_deadlock_spin_limit() -> usize {
        load_usize_override_clamped(
            &IRQSAFE_MUTEX_DEADLOCK_SPIN_LIMIT_OVERRIDE,
            DEFAULT_IRQSAFE_MUTEX_DEADLOCK_SPIN_LIMIT,
            1,
            MAX_IRQSAFE_MUTEX_DEADLOCK_SPIN_LIMIT,
        )
    }

    pub fn set_irqsafe_mutex_deadlock_spin_limit(value: Option<usize>) {
        store_usize_override(&IRQSAFE_MUTEX_DEADLOCK_SPIN_LIMIT_OVERRIDE, value);
    }

    pub fn load_balance_percentile_window() -> usize {
        DEFAULT_LOAD_BALANCE_PERCENTILE_WINDOW
            .max(1)
            .min(MAX_LOAD_BALANCE_PERCENTILE_WINDOW)
    }

    pub fn runtime_policy_drift_reapply_cooldown_ticks() -> u64 {
        load_u64_override_clamped(
            &RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS_OVERRIDE,
            DEFAULT_RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS,
            1,
            MAX_RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS,
        )
    }

    pub fn set_runtime_policy_drift_reapply_cooldown_ticks(value: Option<u64>) {
        store_u64_override(&RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS_OVERRIDE, value);
    }

    pub fn runtime_policy_drift_sample_interval_ticks() -> u64 {
        load_u64_override_clamped(
            &RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS_OVERRIDE,
            DEFAULT_RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS,
            1,
            MAX_RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS,
        )
    }

    pub fn set_runtime_policy_drift_sample_interval_ticks(value: Option<u64>) {
        store_u64_override(&RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS_OVERRIDE, value);
    }

    pub fn runtime_policy_drift_runtime_profile() -> RuntimePolicyDriftRuntimeProfile {
        RuntimePolicyDriftRuntimeProfile {
            sample_interval_ticks: Self::runtime_policy_drift_sample_interval_ticks(),
            reapply_cooldown_ticks: Self::runtime_policy_drift_reapply_cooldown_ticks(),
        }
    }

    pub fn runtime_policy_drift_cargo_profile() -> RuntimePolicyDriftRuntimeProfile {
        RuntimePolicyDriftRuntimeProfile {
            sample_interval_ticks: DEFAULT_RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS.max(1),
            reapply_cooldown_ticks: DEFAULT_RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS.max(1),
        }
    }

    pub fn ahci_io_timeout_spins() -> usize {
        load_usize_override_clamped(
            &AHCI_IO_TIMEOUT_SPINS_OVERRIDE,
            DEFAULT_AHCI_IO_TIMEOUT_SPINS,
            1,
            MAX_DRIVER_WAIT_TIMEOUT_SPINS,
        )
    }

    pub fn nvme_disable_ready_timeout_spins() -> usize {
        load_usize_override_clamped(
            &NVME_DISABLE_READY_TIMEOUT_SPINS_OVERRIDE,
            DEFAULT_NVME_DISABLE_READY_TIMEOUT_SPINS,
            1,
            MAX_DRIVER_WAIT_TIMEOUT_SPINS,
        )
    }

    pub fn nvme_poll_timeout_spins() -> usize {
        load_usize_override_clamped(
            &NVME_POLL_TIMEOUT_SPINS_OVERRIDE,
            DEFAULT_NVME_POLL_TIMEOUT_SPINS,
            1,
            MAX_DRIVER_WAIT_TIMEOUT_SPINS,
        )
    }

    pub fn nvme_io_timeout_spins() -> usize {
        load_usize_override_clamped(
            &NVME_IO_TIMEOUT_SPINS_OVERRIDE,
            DEFAULT_NVME_IO_TIMEOUT_SPINS,
            1,
            MAX_DRIVER_WAIT_TIMEOUT_SPINS,
        )
    }

    pub fn e1000_reset_timeout_spins() -> usize {
        load_usize_override_clamped(
            &E1000_RESET_TIMEOUT_SPINS_OVERRIDE,
            DEFAULT_E1000_RESET_TIMEOUT_SPINS,
            1,
            MAX_DRIVER_WAIT_TIMEOUT_SPINS,
        )
    }

    pub fn e1000_buffer_size_bytes() -> usize {
        load_usize_override_clamped(
            &E1000_BUFFER_SIZE_BYTES_OVERRIDE,
            DEFAULT_E1000_BUFFER_SIZE_BYTES,
            256,
            MAX_E1000_BUFFER_SIZE_BYTES,
        )
    }

    pub fn e1000_rx_desc_count() -> usize {
        let override_value = E1000_RX_DESC_COUNT_OVERRIDE.load(Ordering::Relaxed);
        normalize_e1000_desc_count(override_value, DEFAULT_E1000_RX_DESC_COUNT)
    }

    pub fn e1000_tx_desc_count() -> usize {
        let override_value = E1000_TX_DESC_COUNT_OVERRIDE.load(Ordering::Relaxed);
        normalize_e1000_desc_count(override_value, DEFAULT_E1000_TX_DESC_COUNT)
    }

    pub fn set_ahci_io_timeout_spins(value: Option<usize>) {
        store_usize_override(&AHCI_IO_TIMEOUT_SPINS_OVERRIDE, value);
    }

    pub fn set_nvme_disable_ready_timeout_spins(value: Option<usize>) {
        store_usize_override(&NVME_DISABLE_READY_TIMEOUT_SPINS_OVERRIDE, value);
    }

    pub fn set_nvme_poll_timeout_spins(value: Option<usize>) {
        store_usize_override(&NVME_POLL_TIMEOUT_SPINS_OVERRIDE, value);
    }

    pub fn set_nvme_io_timeout_spins(value: Option<usize>) {
        store_usize_override(&NVME_IO_TIMEOUT_SPINS_OVERRIDE, value);
    }

    pub fn set_e1000_reset_timeout_spins(value: Option<usize>) {
        store_usize_override(&E1000_RESET_TIMEOUT_SPINS_OVERRIDE, value);
    }

    pub fn set_e1000_buffer_size_bytes(value: Option<usize>) {
        store_usize_override(&E1000_BUFFER_SIZE_BYTES_OVERRIDE, value);
    }

    pub fn set_e1000_rx_desc_count(value: Option<usize>) {
        store_usize_override(&E1000_RX_DESC_COUNT_OVERRIDE, value);
    }

    pub fn set_e1000_tx_desc_count(value: Option<usize>) {
        store_usize_override(&E1000_TX_DESC_COUNT_OVERRIDE, value);
    }

    pub fn set_runtime_policy_drift_runtime_profile(
        value: Option<RuntimePolicyDriftRuntimeProfile>,
    ) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_runtime_policy_drift_sample_interval_ticks(Some(
                    profile.sample_interval_ticks,
                ));
                Self::set_runtime_policy_drift_reapply_cooldown_ticks(Some(
                    profile.reapply_cooldown_ticks,
                ));
            },
            || {
                Self::set_runtime_policy_drift_sample_interval_ticks(None);
                Self::set_runtime_policy_drift_reapply_cooldown_ticks(None);
            },
        );
    }
}
