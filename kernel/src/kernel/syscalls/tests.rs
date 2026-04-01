#[cfg(feature = "networking")]
use super::BackpressurePolicyMode;
#[cfg(not(feature = "linux_compat"))]
use super::{
    execve_stack_required_bytes, prepare_execve_user_stack, read_user_c_string_array,
    require_control_plane_access,
};
use super::{
    futex_key_from_ptr_or_hint, parse_process_priority, upcall_entry_pc_valid,
    user_access_range_check_with, user_access_range_valid_with, user_range_valid,
    user_word_aligned, BinarySwitch, CStateOverrideMode, PowerOverrideMode, UserAccessFault,
    UserAccessMode, SYSCALL_ERR_PERMISSION_DENIED,
};
use crate::kernel::syscalls::syscalls_consts::*;

fn mapped_page_index(page: usize) -> usize {
    (page - USER_SPACE_BOTTOM_INCLUSIVE) / PAGE_SIZE
}

fn access_ok_with_windows(
    page: usize,
    mode: UserAccessMode,
    mapped_pages: &[bool],
    writable_pages: &[bool],
) -> bool {
    let idx = mapped_page_index(page);
    if idx >= mapped_pages.len() || !mapped_pages[idx] {
        return false;
    }
    match mode {
        UserAccessMode::Read => true,
        UserAccessMode::Write => writable_pages.get(idx).copied().unwrap_or(false),
    }
}

#[test_case]
fn user_access_cross_page_validation_checks_all_pages() {
    let ptr = USER_SPACE_BOTTOM_INCLUSIVE + PAGE_SIZE - 16;
    let len = 32;
    let mut calls = 0usize;

    let valid = user_access_range_valid_with(ptr, len, UserAccessMode::Read, |_page, mode| {
        assert!(matches!(mode, UserAccessMode::Read));
        calls += 1;
        true
    });

    assert!(valid);
    assert_eq!(calls, 2);
}

#[test_case]
fn user_access_denies_when_second_page_fails() {
    let first_page = USER_SPACE_BOTTOM_INCLUSIVE;
    let second_page = first_page + PAGE_SIZE;
    let ptr = first_page + PAGE_SIZE - 16;
    let len = 32;

    let valid = user_access_range_valid_with(ptr, len, UserAccessMode::Read, |page, _mode| {
        page != second_page
    });

    assert!(!valid);
}

#[test_case]
fn user_access_fault_reports_invalid_range() {
    let res = user_access_range_check_with(0, 32, UserAccessMode::Read, |_page, _mode| None);
    assert_eq!(res, Err(UserAccessFault::InvalidRange));
}

#[test_case]
fn user_access_fault_reports_not_writable() {
    let ptr = USER_SPACE_BOTTOM_INCLUSIVE + 64;
    let len = 128;

    let res = user_access_range_check_with(ptr, len, UserAccessMode::Write, |_page, _mode| {
        Some(UserAccessFault::NotWritable)
    });

    assert_eq!(res, Err(UserAccessFault::NotWritable));
}

#[test_case]
fn user_access_fault_reports_not_present() {
    let ptr = USER_SPACE_BOTTOM_INCLUSIVE + PAGE_SIZE - 32;
    let len = 64;

    let res = user_access_range_check_with(ptr, len, UserAccessMode::Read, |page, _mode| {
        if page == USER_SPACE_BOTTOM_INCLUSIVE + PAGE_SIZE {
            Some(UserAccessFault::NotPresent)
        } else {
            None
        }
    });

    assert_eq!(res, Err(UserAccessFault::NotPresent));
}

#[test_case]
fn user_access_denies_write_permission_failures() {
    let ptr = USER_SPACE_BOTTOM_INCLUSIVE + 128;
    let len = 64;

    let read_ok = user_access_range_valid_with(ptr, len, UserAccessMode::Read, |_page, mode| {
        matches!(mode, UserAccessMode::Read)
    });
    let write_denied =
        user_access_range_valid_with(ptr, len, UserAccessMode::Write, |_page, mode| {
            matches!(mode, UserAccessMode::Read)
        });

    assert!(read_ok);
    assert!(!write_denied);
}

#[test_case]
fn user_range_rejects_zero_len() {
    assert!(!user_range_valid(USER_SPACE_BOTTOM_INCLUSIVE, 0));
}

#[test_case]
fn user_range_rejects_overflow() {
    assert!(!user_range_valid(usize::MAX - 4, 16));
}

#[test_case]
fn user_range_rejects_below_user_bottom() {
    assert!(!user_range_valid(USER_SPACE_BOTTOM_INCLUSIVE - 1, 1));
}

#[test_case]
fn user_word_alignment_guard_works() {
    assert!(user_word_aligned(USER_SPACE_BOTTOM_INCLUSIVE));
    assert!(!user_word_aligned(USER_SPACE_BOTTOM_INCLUSIVE + 1));
}

#[test_case]
fn binary_switch_parser_rejects_invalid_values() {
    assert!(BinarySwitch::from_usize(0).is_some());
    assert!(BinarySwitch::from_usize(1).is_some());
    assert!(BinarySwitch::from_usize(2).is_none());
}

#[cfg(feature = "networking")]
#[test_case]
fn backpressure_policy_parser_rejects_invalid_values() {
    assert!(BackpressurePolicyMode::from_usize(0).is_some());
    assert!(BackpressurePolicyMode::from_usize(1).is_some());
    assert!(BackpressurePolicyMode::from_usize(2).is_some());
    assert!(BackpressurePolicyMode::from_usize(3).is_none());
}

#[test_case]
fn power_and_cstate_mode_parsers_reject_invalid_values() {
    assert!(PowerOverrideMode::from_usize(0).is_some());
    assert!(PowerOverrideMode::from_usize(1).is_some());
    assert!(PowerOverrideMode::from_usize(2).is_some());
    assert!(PowerOverrideMode::from_usize(3).is_none());

    assert!(CStateOverrideMode::from_usize(0).is_some());
    assert!(CStateOverrideMode::from_usize(1).is_some());
    assert!(CStateOverrideMode::from_usize(2).is_some());
    assert!(CStateOverrideMode::from_usize(3).is_none());
}

#[test_case]
fn launch_context_word_layout_is_stable() {
    let mut out = [0usize; 8];
    write_launch_context_words(&mut out, 1, 2, 3, 4, 5, 6, 7, 8);
    assert_eq!(out, [1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test_case]
fn process_priority_parser_rejects_out_of_range() {
    assert_eq!(parse_process_priority(0), Some(0));
    assert_eq!(parse_process_priority(255), Some(255));
    assert_eq!(parse_process_priority(256), None);
}

#[test_case]
fn bounded_user_read_rejects_zero_length() {
    let limit = crate::config::KernelConfig::launch_max_process_name_len();
    let res = super::with_user_read_bounded_bytes(0, 0, limit, |bytes| bytes.len());
    assert!(res.is_err());
}

#[test_case]
fn bounded_user_read_rejects_length_above_limit() {
    let limit = crate::config::KernelConfig::launch_max_process_name_len();
    let res = super::with_user_read_bounded_bytes(0, limit + 1, limit, |bytes| bytes.len());
    assert!(res.is_err());
}

#[test_case]
fn user_access_matrix_covers_overflow_alignment_and_permission_cases() {
    let cases = [
        (
            0usize,
            1usize,
            UserAccessMode::Read,
            Err(UserAccessFault::InvalidRange),
        ),
        (
            USER_SPACE_TOP_EXCLUSIVE - 8,
            16usize,
            UserAccessMode::Read,
            Err(UserAccessFault::InvalidRange),
        ),
        (
            USER_SPACE_BOTTOM_INCLUSIVE + PAGE_SIZE - 8,
            16usize,
            UserAccessMode::Write,
            Err(UserAccessFault::NotWritable),
        ),
    ];

    for (ptr, len, mode, expected) in cases {
        let res = user_access_range_check_with(ptr, len, mode, |page, access_mode| {
            if matches!(access_mode, UserAccessMode::Write)
                && page == USER_SPACE_BOTTOM_INCLUSIVE + PAGE_SIZE
            {
                Some(UserAccessFault::NotWritable)
            } else {
                None
            }
        });
        assert_eq!(res, expected);
    }

    assert!(user_word_aligned(USER_SPACE_BOTTOM_INCLUSIVE));
    assert!(!user_word_aligned(USER_SPACE_BOTTOM_INCLUSIVE + 1));
}

#[cfg(not(feature = "linux_compat"))]
#[test_case]
fn process_spawn_access_gate_denies_when_mac_clearance_is_low() {
    use crate::interfaces::security::SecurityLevel;
    use crate::modules::security::{MacLabel, RESOURCE_PROCESS_SPAWN};

    crate::modules::security::mac::set_resource_security_level(
        RESOURCE_PROCESS_SPAWN,
        SecurityLevel::TopSecret,
    );
    crate::modules::security::set_mac_subject_clearance(MacLabel::Confidential);

    let denied = require_control_plane_access(RESOURCE_PROCESS_SPAWN);
    assert_eq!(denied, Err(SYSCALL_ERR_PERMISSION_DENIED));

    crate::modules::security::set_mac_subject_clearance(MacLabel::TopSecret);
    crate::modules::security::mac::set_resource_security_level(
        RESOURCE_PROCESS_SPAWN,
        SecurityLevel::Unclassified,
    );
}

#[cfg(not(feature = "linux_compat"))]
#[test_case]
fn process_kill_access_gate_denies_when_mac_clearance_is_low() {
    use crate::interfaces::security::SecurityLevel;
    use crate::modules::security::{MacLabel, RESOURCE_PROCESS_KILL};

    crate::modules::security::mac::set_resource_security_level(
        RESOURCE_PROCESS_KILL,
        SecurityLevel::TopSecret,
    );
    crate::modules::security::set_mac_subject_clearance(MacLabel::Confidential);

    let denied = require_control_plane_access(RESOURCE_PROCESS_KILL);
    assert_eq!(denied, Err(SYSCALL_ERR_PERMISSION_DENIED));

    crate::modules::security::set_mac_subject_clearance(MacLabel::TopSecret);
    crate::modules::security::mac::set_resource_security_level(
        RESOURCE_PROCESS_KILL,
        SecurityLevel::Unclassified,
    );
}

#[cfg(not(feature = "linux_compat"))]
#[test_case]
fn execve_stack_helper_rejects_tiny_stack_before_user_write() {
    let argv = alloc::vec![alloc::string::String::from("arg0")];
    let envp = alloc::vec![alloc::string::String::from("k=v")];
    let err = prepare_execve_user_stack(USER_SPACE_BOTTOM_INCLUSIVE as u64, 8, &argv, &envp, &[])
        .unwrap_err();
    assert_eq!(
        err,
        (-(crate::modules::posix_consts::errno::EINVAL as isize)) as usize
    );
}

#[cfg(not(feature = "linux_compat"))]
#[test_case]
fn execve_stack_size_helper_includes_strings_pointers_and_alignment() {
    let argv = alloc::vec![
        alloc::string::String::from("prog"),
        alloc::string::String::from("--flag"),
    ];
    let envp = alloc::vec![alloc::string::String::from("A=B")];
    let required = execve_stack_required_bytes(&argv, &envp, &[]).unwrap();
    let string_bytes = ("prog".len() + 1) + ("--flag".len() + 1) + ("A=B".len() + 1);
    assert!(required > string_bytes);
}

#[cfg(not(feature = "linux_compat"))]
#[test_case]
fn c_string_array_reader_allows_null_pointer_as_empty_vector() {
    let items = read_user_c_string_array(0, 16, USER_CSTRING_MAX_LEN).unwrap();
    assert!(items.is_empty());
}

#[cfg(feature = "vfs")]
#[test_case]
fn vfs_path_wrapper_rejects_zero_length() {
    let res = super::with_user_vfs_path(0, 0, |path| path.len());
    assert!(res.is_err());
}

#[cfg(feature = "vfs")]
#[test_case]
fn mount_record_serializer_layout_is_stable() {
    let record = crate::kernel::vfs_control::MountRecord {
        id: 11,
        fs_kind: 22,
        path_len: 33,
    };
    let mut out = [0usize; 3];
    super::write_mount_record_words(&mut out, 0, &record);
    assert_eq!(out, [11, 22, 33]);
}

#[test_case]
fn futex_key_uses_pointer_when_hint_is_zero() {
    assert_eq!(futex_key_from_ptr_or_hint(0x1234, 0), 0x1234);
}

#[test_case]
fn futex_key_uses_hint_when_provided() {
    assert_eq!(futex_key_from_ptr_or_hint(0x1234, 0x5555), 0x5555);
}

#[test_case]
fn upcall_entry_pc_validation_enforces_user_space_bounds() {
    assert!(!upcall_entry_pc_valid(USER_SPACE_BOTTOM_INCLUSIVE - 1));
    assert!(upcall_entry_pc_valid(USER_SPACE_BOTTOM_INCLUSIVE));
    assert!(upcall_entry_pc_valid(USER_SPACE_TOP_EXCLUSIVE - 1));
    assert!(!upcall_entry_pc_valid(USER_SPACE_TOP_EXCLUSIVE));
}

#[test_case]
fn upcall_query_word_count_stays_stable() {
    assert_eq!(UPCALL_QUERY_WORDS, 4);
}

#[test_case]
fn upcall_delivery_word_count_stays_stable() {
    assert_eq!(UPCALL_DELIVERY_WORDS, 5);
}

#[test_case]
fn syscall_abi_contract_constants_are_stable() {
    assert_eq!(SYSCALL_ABI_INFO_WORDS, 7);
    assert_eq!(SYSCALL_ABI_MAGIC, 0x48594241);
    assert!(SYSCALL_ABI_VERSION_MAJOR >= 1);
    assert_eq!(super::nr::GET_ABI_INFO, 45);
    assert_eq!(super::nr::SET_NETWORK_BACKPRESSURE_POLICY, 46);
    assert_eq!(super::nr::SET_NETWORK_ALERT_THRESHOLDS, 47);
    assert_eq!(super::nr::GET_NETWORK_ALERT_REPORT, 48);
    assert_eq!(super::nr::GET_CRASH_REPORT, 54);
    assert_eq!(super::nr::LIST_CRASH_EVENTS, 55);
    assert_eq!(super::nr::GET_CORE_PRESSURE_SNAPSHOT, 56);
    assert_eq!(super::nr::GET_LOTTERY_REPLAY_LATEST, 57);
    assert_eq!(super::nr::SET_POLICY_DRIFT_CONTROL, 58);
    assert_eq!(super::nr::GET_POLICY_DRIFT_CONTROL, 59);
    assert_eq!(super::nr::GET_POLICY_DRIFT_REASON_TEXT, 60);
    assert_eq!(CRASH_REPORT_WORDS, 10);
    assert_eq!(CRASH_EVENT_WORDS, 8);
    assert_eq!(CORE_PRESSURE_SNAPSHOT_WORDS, 18);
    assert_eq!(LOTTERY_REPLAY_LATEST_WORDS, 5);
}

#[test_case]
fn core_pressure_class_encodings_are_stable() {
    use crate::kernel::pressure::{CorePressureClass, SchedulerPressureClass};

    assert_eq!(
        super::encode_core_pressure_class(CorePressureClass::Nominal),
        0
    );
    assert_eq!(
        super::encode_core_pressure_class(CorePressureClass::Elevated),
        1
    );
    assert_eq!(
        super::encode_core_pressure_class(CorePressureClass::High),
        2
    );
    assert_eq!(
        super::encode_core_pressure_class(CorePressureClass::Critical),
        3
    );

    assert_eq!(
        super::encode_scheduler_pressure_class(SchedulerPressureClass::Nominal),
        0
    );
    assert_eq!(
        super::encode_scheduler_pressure_class(SchedulerPressureClass::Elevated),
        1
    );
    assert_eq!(
        super::encode_scheduler_pressure_class(SchedulerPressureClass::High),
        2
    );
    assert_eq!(
        super::encode_scheduler_pressure_class(SchedulerPressureClass::Critical),
        3
    );
}

#[test_case]
fn core_pressure_snapshot_word_layout_is_stable() {
    use crate::kernel::pressure::{
        CorePressureClass, CorePressureSnapshot, SchedulerPressureClass,
    };

    let pressure = CorePressureSnapshot {
        schema_version: 2,
        online_cpus: 8,
        runqueue_total: 10,
        runqueue_max: 4,
        runqueue_avg_milli: 1250,
        rt_starvation_alert: true,
        rt_forced_reschedules: 12,
        watchdog_stall_detections: 1,
        net_queue_limit: 1024,
        net_rx_depth: 40,
        net_tx_depth: 20,
        net_saturation_percent: 3,
        lb_imbalance_p50: 2,
        lb_imbalance_p90: 5,
        lb_imbalance_p99: 8,
        lb_prefer_local_forced_moves: 7,
        class: CorePressureClass::Elevated,
        scheduler_class: SchedulerPressureClass::High,
    };

    let mut out = [0usize; CORE_PRESSURE_SNAPSHOT_WORDS];
    super::write_core_pressure_snapshot_words(&mut out, pressure);

    assert_eq!(
        out,
        [2, 8, 10, 4, 1250, 1, 12, 1, 1024, 40, 20, 3, 2, 5, 8, 7, 1, 2]
    );
}

#[path = "tests_stress.rs"]
mod tests_stress;
