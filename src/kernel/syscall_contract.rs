#[cfg(all(feature = "linux_compat", feature = "ring_protection"))]
use crate::kernel::syscalls::syscalls_consts::linux_nr;
use crate::kernel::syscalls::syscalls_consts::{
    nr, CORE_PRESSURE_SNAPSHOT_WORDS, LOTTERY_REPLAY_LATEST_WORDS, SYSCALL_ABI_FLAG_STABLE,
    SYSCALL_ABI_INFO_WORDS, SYSCALL_ABI_MAGIC, SYSCALL_ABI_MIN_COMPAT_MAJOR,
    SYSCALL_ABI_VERSION_MAJOR, SYSCALL_ABI_VERSION_MINOR, SYSCALL_ABI_VERSION_PATCH,
};

#[derive(Debug, Clone, Copy)]
pub struct SyscallContractReport {
    pub checks: u32,
    pub failures: u32,
    pub last_error_code: u32,
}

impl SyscallContractReport {
    #[inline(always)]
    pub const fn passed(self) -> bool {
        self.failures == 0
    }
}

pub fn run_syscall_contract_self_test() -> SyscallContractReport {
    let mut checks = 0u32;
    let mut failures = 0u32;
    let mut last_error_code = 0u32;

    macro_rules! check {
        ($code:expr, $cond:expr, $msg:expr) => {{
            checks = checks.saturating_add(1);
            if !($cond) {
                failures = failures.saturating_add(1);
                last_error_code = $code;
                crate::klog_error!("[SYSCALL CONTRACT] E{}: {}", $code, $msg);
            }
        }};
    }

    check!(
        2001,
        SYSCALL_ABI_INFO_WORDS == 7,
        "SYSCALL_ABI_INFO_WORDS must stay 7"
    );
    check!(
        2002,
        SYSCALL_ABI_MAGIC == 0x48594241,
        "SYSCALL_ABI_MAGIC mismatch"
    );
    check!(
        2003,
        SYSCALL_ABI_VERSION_MAJOR >= 1,
        "SYSCALL_ABI_VERSION_MAJOR must be >= 1"
    );
    check!(
        2004,
        SYSCALL_ABI_MIN_COMPAT_MAJOR <= SYSCALL_ABI_VERSION_MAJOR,
        "SYSCALL_ABI_MIN_COMPAT_MAJOR must be <= ABI major"
    );
    check!(
        2005,
        (SYSCALL_ABI_FLAG_STABLE & 0x1) == 1,
        "stable ABI flag must keep bit0"
    );
    check!(
        2006,
        nr::GET_ABI_INFO == 45,
        "core GET_ABI_INFO nr must remain 45"
    );
    check!(
        2007,
        nr::RESOLVE_PLT == 49,
        "core RESOLVE_PLT nr must remain 49"
    );
    check!(
        2008,
        nr::VFS_CLOSE == 53,
        "core VFS_CLOSE nr must remain 53"
    );
    check!(
        2013,
        nr::GET_CRASH_REPORT == 54,
        "core GET_CRASH_REPORT nr must remain 54"
    );
    check!(
        2014,
        nr::LIST_CRASH_EVENTS == 55,
        "core LIST_CRASH_EVENTS nr must remain 55"
    );
    check!(
        2015,
        nr::GET_CORE_PRESSURE_SNAPSHOT == 56,
        "core GET_CORE_PRESSURE_SNAPSHOT nr must remain 56"
    );
    check!(
        2016,
        CORE_PRESSURE_SNAPSHOT_WORDS == 18,
        "CORE_PRESSURE_SNAPSHOT_WORDS must stay 18"
    );
    check!(
        2017,
        nr::GET_LOTTERY_REPLAY_LATEST == 57,
        "core GET_LOTTERY_REPLAY_LATEST nr must remain 57"
    );
    check!(
        2018,
        LOTTERY_REPLAY_LATEST_WORDS == 5,
        "LOTTERY_REPLAY_LATEST_WORDS must stay 5"
    );
    check!(
        2019,
        nr::SET_POLICY_DRIFT_CONTROL == 58,
        "core SET_POLICY_DRIFT_CONTROL nr must remain 58"
    );
    check!(
        2020,
        nr::GET_POLICY_DRIFT_CONTROL == 59,
        "core GET_POLICY_DRIFT_CONTROL nr must remain 59"
    );
    check!(
        2021,
        nr::GET_POLICY_DRIFT_REASON_TEXT == 60,
        "core GET_POLICY_DRIFT_REASON_TEXT nr must remain 60"
    );
    check!(
        2009,
        SYSCALL_ABI_VERSION_MINOR <= 1024 && SYSCALL_ABI_VERSION_PATCH <= 4096,
        "ABI minor/patch sanity bounds failed"
    );

    let core_nrs = [
        nr::YIELD,
        nr::EXIT,
        nr::PRINT,
        nr::SET_TLS,
        nr::GET_TLS,
        nr::SET_AFFINITY,
        nr::GET_AFFINITY,
        nr::GET_LAUNCH_STATS,
        nr::GET_PROCESS_COUNT,
        nr::LIST_PROCESS_IDS,
        nr::SPAWN_PROCESS,
        nr::GET_PROCESS_IMAGE_STATE,
        nr::GET_PROCESS_MAPPING_STATE,
        nr::VFS_MOUNT_RAMFS,
        nr::VFS_LIST_MOUNTS,
        nr::GET_POWER_STATS,
        nr::SET_POWER_OVERRIDE,
        nr::CLEAR_POWER_OVERRIDE,
        nr::GET_NETWORK_STATS,
        nr::SET_NETWORK_POLLING,
        nr::TERMINATE_PROCESS,
        nr::GET_PROCESS_LAUNCH_CONTEXT,
        nr::VFS_GET_MOUNT_PATH,
        nr::VFS_UNMOUNT,
        nr::VFS_GET_STATS,
        nr::NETWORK_RESET_STATS,
        nr::NETWORK_FORCE_POLL,
        nr::SET_CSTATE_OVERRIDE,
        nr::CLEAR_CSTATE_OVERRIDE,
        nr::CLAIM_NEXT_LAUNCH_CONTEXT,
        nr::ACK_LAUNCH_CONTEXT,
        nr::GET_LAUNCH_CONTEXT_STAGE,
        nr::TERMINATE_TASK,
        nr::GET_PROCESS_ID_BY_TASK,
        nr::VFS_UNMOUNT_PATH,
        nr::NETWORK_REINITIALIZE,
        nr::CONSUME_READY_LAUNCH_CONTEXT,
        nr::EXECUTE_READY_LAUNCH_CONTEXT,
        nr::FUTEX_WAIT,
        nr::FUTEX_WAKE,
        nr::UPCALL_REGISTER,
        nr::UPCALL_UNREGISTER,
        nr::UPCALL_QUERY,
        nr::UPCALL_CONSUME,
        nr::UPCALL_INJECT_VIRQ,
        nr::GET_ABI_INFO,
        nr::SET_NETWORK_BACKPRESSURE_POLICY,
        nr::SET_NETWORK_ALERT_THRESHOLDS,
        nr::GET_NETWORK_ALERT_REPORT,
        nr::RESOLVE_PLT,
        nr::VFS_OPEN,
        nr::VFS_READ,
        nr::VFS_WRITE,
        nr::VFS_CLOSE,
        nr::GET_CRASH_REPORT,
        nr::LIST_CRASH_EVENTS,
        nr::GET_CORE_PRESSURE_SNAPSHOT,
        nr::GET_LOTTERY_REPLAY_LATEST,
        nr::SET_POLICY_DRIFT_CONTROL,
        nr::GET_POLICY_DRIFT_CONTROL,
        nr::GET_POLICY_DRIFT_REASON_TEXT,
    ];
    let strictly_increasing = core_nrs.windows(2).all(|w| w[0] < w[1]);
    check!(
        2010,
        strictly_increasing,
        "core syscall nr list must stay strictly increasing"
    );

    #[cfg(all(feature = "linux_compat", feature = "ring_protection"))]
    {
        let mut frame = crate::kernel::syscalls::SyscallFrame::default();
        let getpid_mapped = crate::modules::linux_compat::sys_dispatcher::sys_linux_compat(
            linux_nr::GETPID,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut frame,
        )
        .is_some();
        check!(2011, getpid_mapped, "linux dispatcher must map GETPID");

        let unknown_returns_none = crate::modules::linux_compat::sys_dispatcher::sys_linux_compat(
            usize::MAX,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut frame,
        )
        .is_none();
        check!(
            2012,
            unknown_returns_none,
            "linux dispatcher unknown syscall should return None"
        );
    }

    if failures == 0 {
        crate::klog_info!("[SYSCALL CONTRACT] passed checks={}", checks);
    } else {
        crate::klog_error!(
            "[SYSCALL CONTRACT] failed checks={} failures={} last_error=E{}",
            checks,
            failures,
            last_error_code
        );
    }

    SyscallContractReport {
        checks,
        failures,
        last_error_code,
    }
}
