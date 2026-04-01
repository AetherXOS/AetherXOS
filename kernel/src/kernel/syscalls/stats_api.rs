use super::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct SyscallStats {
    pub total: u64,
    pub unknown: u64,
    pub invalid_args: u64,
    pub user_access_denied: u64,
    pub user_word_unaligned_denied: u64,
    pub print_calls: u64,
    pub tls_calls: u64,
    pub affinity_calls: u64,
    pub abi_info_calls: u64,
    pub launch_stats_calls: u64,
    pub process_count_calls: u64,
    pub process_list_calls: u64,
    pub process_spawn_calls: u64,
    pub process_image_state_calls: u64,
    pub process_mapping_state_calls: u64,
    pub vfs_mount_ramfs_calls: u64,
    pub vfs_mount_diskfs_calls: u64,
    pub vfs_list_mounts_calls: u64,
    pub power_stats_calls: u64,
    pub power_override_set_calls: u64,
    pub power_override_clear_calls: u64,
    pub network_stats_calls: u64,
    pub network_poll_control_calls: u64,
    pub process_terminate_calls: u64,
    pub process_launch_ctx_calls: u64,
    pub vfs_mount_path_calls: u64,
    pub vfs_unmount_calls: u64,
    pub vfs_stats_calls: u64,
    pub network_reset_stats_calls: u64,
    pub network_force_poll_calls: u64,
    pub power_cstate_set_calls: u64,
    pub power_cstate_clear_calls: u64,
    pub process_claim_ctx_calls: u64,
    pub process_ack_ctx_calls: u64,
    pub process_ctx_stage_calls: u64,
    pub task_terminate_calls: u64,
    pub task_process_id_calls: u64,
    pub vfs_unmount_path_calls: u64,
    pub network_reinit_calls: u64,
    pub process_consume_ctx_calls: u64,
    pub process_execute_ctx_calls: u64,
    pub futex_wait_calls: u64,
    pub futex_wake_calls: u64,
    pub upcall_register_calls: u64,
    pub upcall_unregister_calls: u64,
    pub upcall_query_calls: u64,
    pub upcall_consume_calls: u64,
    pub upcall_inject_virq_calls: u64,
    pub network_backpressure_policy_calls: u64,
    pub network_alert_thresholds_calls: u64,
    pub network_alert_report_calls: u64,
    pub crash_report_calls: u64,
    pub crash_events_calls: u64,
    pub core_pressure_snapshot_calls: u64,
    pub lottery_replay_latest_calls: u64,
    pub policy_drift_control_set_calls: u64,
    pub policy_drift_control_get_calls: u64,
    pub policy_drift_reason_text_calls: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallHealthAction {
    None,
    AuditUnknownSyscalls,
    TightenUserPointerValidation,
}

#[derive(Debug, Clone, Copy)]
pub struct SyscallHealthReport {
    pub total: u64,
    pub unknown_rate_per_mille: u64,
    pub invalid_arg_rate_per_mille: u64,
    pub user_access_denied_rate_per_mille: u64,
    pub control_plane_ratio_per_mille: u64,
    pub degraded: bool,
}

#[inline(always)]
fn ratio_per_mille(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        0
    } else {
        numerator.saturating_mul(1000).saturating_div(denominator)
    }
}

pub fn evaluate_syscall_health(stats: SyscallStats) -> SyscallHealthReport {
    let control_plane_calls = stats
        .network_stats_calls
        .saturating_add(stats.vfs_stats_calls)
        .saturating_add(stats.power_stats_calls)
        .saturating_add(stats.process_count_calls)
        .saturating_add(stats.process_list_calls)
        .saturating_add(stats.core_pressure_snapshot_calls)
        .saturating_add(stats.lottery_replay_latest_calls);

    let unknown_rate = ratio_per_mille(stats.unknown, stats.total);
    let invalid_arg_rate = ratio_per_mille(stats.invalid_args, stats.total);
    let denied_rate = ratio_per_mille(stats.user_access_denied, stats.total);
    let control_plane_ratio = ratio_per_mille(control_plane_calls, stats.total);
    let degraded = unknown_rate > 50 || invalid_arg_rate > 100 || denied_rate > 120;

    SyscallHealthReport {
        total: stats.total,
        unknown_rate_per_mille: unknown_rate,
        invalid_arg_rate_per_mille: invalid_arg_rate,
        user_access_denied_rate_per_mille: denied_rate,
        control_plane_ratio_per_mille: control_plane_ratio,
        degraded,
    }
}

pub fn recommended_syscall_health_action(report: SyscallHealthReport) -> SyscallHealthAction {
    if report.unknown_rate_per_mille > 50 {
        return SyscallHealthAction::AuditUnknownSyscalls;
    }
    if report.invalid_arg_rate_per_mille > 100 || report.user_access_denied_rate_per_mille > 120 {
        return SyscallHealthAction::TightenUserPointerValidation;
    }
    SyscallHealthAction::None
}

pub fn current_syscall_health() -> SyscallHealthReport {
    evaluate_syscall_health(stats())
}

pub fn stats() -> SyscallStats {
    SyscallStats {
        total: SYSCALL_TOTAL.load(Ordering::Relaxed),
        unknown: SYSCALL_UNKNOWN.load(Ordering::Relaxed),
        invalid_args: SYSCALL_INVALID_ARGS.load(Ordering::Relaxed),
        user_access_denied: SYSCALL_USER_ACCESS_DENIED.load(Ordering::Relaxed),
        user_word_unaligned_denied: SYSCALL_USER_WORD_UNALIGNED_DENIED.load(Ordering::Relaxed),
        print_calls: SYSCALL_PRINT_CALLS.load(Ordering::Relaxed),
        tls_calls: SYSCALL_TLS_CALLS.load(Ordering::Relaxed),
        affinity_calls: SYSCALL_AFFINITY_CALLS.load(Ordering::Relaxed),
        abi_info_calls: SYSCALL_ABI_INFO_CALLS.load(Ordering::Relaxed),
        launch_stats_calls: SYSCALL_LAUNCH_STATS_CALLS.load(Ordering::Relaxed),
        process_count_calls: SYSCALL_PROCESS_COUNT_CALLS.load(Ordering::Relaxed),
        process_list_calls: SYSCALL_PROCESS_LIST_CALLS.load(Ordering::Relaxed),
        process_spawn_calls: SYSCALL_PROCESS_SPAWN_CALLS.load(Ordering::Relaxed),
        process_image_state_calls: SYSCALL_PROCESS_IMAGE_STATE_CALLS.load(Ordering::Relaxed),
        process_mapping_state_calls: SYSCALL_PROCESS_MAPPING_STATE_CALLS.load(Ordering::Relaxed),
        vfs_mount_ramfs_calls: SYSCALL_VFS_MOUNT_RAMFS_CALLS.load(Ordering::Relaxed),
        vfs_mount_diskfs_calls: SYSCALL_VFS_MOUNT_DISKFS_CALLS.load(Ordering::Relaxed),
        vfs_list_mounts_calls: SYSCALL_VFS_LIST_MOUNTS_CALLS.load(Ordering::Relaxed),
        power_stats_calls: SYSCALL_POWER_STATS_CALLS.load(Ordering::Relaxed),
        power_override_set_calls: SYSCALL_POWER_OVERRIDE_SET_CALLS.load(Ordering::Relaxed),
        power_override_clear_calls: SYSCALL_POWER_OVERRIDE_CLEAR_CALLS.load(Ordering::Relaxed),
        network_stats_calls: SYSCALL_NETWORK_STATS_CALLS.load(Ordering::Relaxed),
        network_poll_control_calls: SYSCALL_NETWORK_POLL_CONTROL_CALLS.load(Ordering::Relaxed),
        process_terminate_calls: SYSCALL_PROCESS_TERMINATE_CALLS.load(Ordering::Relaxed),
        process_launch_ctx_calls: SYSCALL_PROCESS_LAUNCH_CTX_CALLS.load(Ordering::Relaxed),
        vfs_mount_path_calls: SYSCALL_VFS_MOUNT_PATH_CALLS.load(Ordering::Relaxed),
        vfs_unmount_calls: SYSCALL_VFS_UNMOUNT_CALLS.load(Ordering::Relaxed),
        vfs_stats_calls: SYSCALL_VFS_STATS_CALLS.load(Ordering::Relaxed),
        network_reset_stats_calls: SYSCALL_NETWORK_RESET_STATS_CALLS.load(Ordering::Relaxed),
        network_force_poll_calls: SYSCALL_NETWORK_FORCE_POLL_CALLS.load(Ordering::Relaxed),
        power_cstate_set_calls: SYSCALL_POWER_CSTATE_SET_CALLS.load(Ordering::Relaxed),
        power_cstate_clear_calls: SYSCALL_POWER_CSTATE_CLEAR_CALLS.load(Ordering::Relaxed),
        process_claim_ctx_calls: SYSCALL_PROCESS_CLAIM_CTX_CALLS.load(Ordering::Relaxed),
        process_ack_ctx_calls: SYSCALL_PROCESS_ACK_CTX_CALLS.load(Ordering::Relaxed),
        process_ctx_stage_calls: SYSCALL_PROCESS_CTX_STAGE_CALLS.load(Ordering::Relaxed),
        task_terminate_calls: SYSCALL_TASK_TERMINATE_CALLS.load(Ordering::Relaxed),
        task_process_id_calls: SYSCALL_TASK_PROCESS_ID_CALLS.load(Ordering::Relaxed),
        vfs_unmount_path_calls: SYSCALL_VFS_UNMOUNT_PATH_CALLS.load(Ordering::Relaxed),
        network_reinit_calls: SYSCALL_NETWORK_REINIT_CALLS.load(Ordering::Relaxed),
        process_consume_ctx_calls: SYSCALL_PROCESS_CONSUME_CTX_CALLS.load(Ordering::Relaxed),
        process_execute_ctx_calls: SYSCALL_PROCESS_EXECUTE_CTX_CALLS.load(Ordering::Relaxed),
        futex_wait_calls: SYSCALL_FUTEX_WAIT_CALLS.load(Ordering::Relaxed),
        futex_wake_calls: SYSCALL_FUTEX_WAKE_CALLS.load(Ordering::Relaxed),
        upcall_register_calls: SYSCALL_UPCALL_REGISTER_CALLS.load(Ordering::Relaxed),
        upcall_unregister_calls: SYSCALL_UPCALL_UNREGISTER_CALLS.load(Ordering::Relaxed),
        upcall_query_calls: SYSCALL_UPCALL_QUERY_CALLS.load(Ordering::Relaxed),
        upcall_consume_calls: SYSCALL_UPCALL_CONSUME_CALLS.load(Ordering::Relaxed),
        upcall_inject_virq_calls: SYSCALL_UPCALL_INJECT_VIRQ_CALLS.load(Ordering::Relaxed),
        network_backpressure_policy_calls: SYSCALL_NETWORK_BACKPRESSURE_POLICY_CALLS
            .load(Ordering::Relaxed),
        network_alert_thresholds_calls: SYSCALL_NETWORK_ALERT_THRESHOLDS_CALLS
            .load(Ordering::Relaxed),
        network_alert_report_calls: SYSCALL_NETWORK_ALERT_REPORT_CALLS.load(Ordering::Relaxed),
        crash_report_calls: SYSCALL_CRASH_REPORT_CALLS.load(Ordering::Relaxed),
        crash_events_calls: SYSCALL_CRASH_EVENTS_CALLS.load(Ordering::Relaxed),
        core_pressure_snapshot_calls: SYSCALL_CORE_PRESSURE_SNAPSHOT_CALLS.load(Ordering::Relaxed),
        lottery_replay_latest_calls: SYSCALL_LOTTERY_REPLAY_LATEST_CALLS.load(Ordering::Relaxed),
        policy_drift_control_set_calls: SYSCALL_POLICY_DRIFT_CONTROL_SET_CALLS
            .load(Ordering::Relaxed),
        policy_drift_control_get_calls: SYSCALL_POLICY_DRIFT_CONTROL_GET_CALLS
            .load(Ordering::Relaxed),
        policy_drift_reason_text_calls: SYSCALL_POLICY_DRIFT_REASON_TEXT_CALLS
            .load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn syscall_health_detects_unknown_spike() {
        let mut s = SyscallStats::default();
        s.total = 1000;
        s.unknown = 90;
        let report = evaluate_syscall_health(s);
        assert!(report.degraded);
        assert_eq!(
            recommended_syscall_health_action(report),
            SyscallHealthAction::AuditUnknownSyscalls
        );
    }

    #[test_case]
    fn syscall_health_prefers_pointer_validation_on_access_denials() {
        let mut s = SyscallStats::default();
        s.total = 1000;
        s.user_access_denied = 300;
        let report = evaluate_syscall_health(s);
        assert_eq!(
            recommended_syscall_health_action(report),
            SyscallHealthAction::TightenUserPointerValidation
        );
    }
}
