#[cfg(target_arch = "x86_64")]
pub const USER_SPACE_TOP_EXCLUSIVE: usize = 0x0000_8000_0000_0000;
#[cfg(target_arch = "aarch64")]
pub const USER_SPACE_TOP_EXCLUSIVE: usize = 0x0001_0000_0000_0000; // 48-bit VA
pub const USER_SPACE_BOTTOM_INCLUSIVE: usize = 0x1000;
pub const PROCESS_PRIORITY_MAX: usize = u8::MAX as usize;
pub const PAGE_SIZE: usize = 4096;
pub const STDIN_FD: usize = 0;
pub const STDOUT_FD: usize = 1;
pub const STDERR_FD: usize = 2;
pub const SEEK_SET: usize = 0;
pub const SEEK_CUR: usize = 1;
pub const SEEK_END: usize = 2;
pub const FUTEX_WAIT_OP: usize = 0;
pub const FUTEX_WAKE_OP: usize = 1;

pub mod x86 {
    pub const CPU_LOCAL_SCRATCH: usize = 8;
    pub const CPU_LOCAL_KSTACK_TOP: usize = 16;
    pub const CPU_LOCAL_CURRENT_TASK: usize = 24;

    pub const RFLAGS_IF: u64 = 0x200;
    pub const RFLAGS_RESERVED: u64 = 0x2;
    pub const RFLAGS_IF_RESERVED: u64 = RFLAGS_IF | RFLAGS_RESERVED;

    pub const IRQ_VECTOR_BASE: u8 = 32;
    pub const IRQ_TIMER: u8 = IRQ_VECTOR_BASE;
    pub const IRQ_TLB_SHOOTDOWN: u8 = 253;
    pub const EXCEPTION_GPF: u8 = 13;
    pub const EXCEPTION_PAGE_FAULT: u8 = 14;
    pub const EXCEPTION_BREAKPOINT: u8 = 3;
    pub const EXCEPTION_DOUBLE_FAULT: u8 = 8;
}

pub mod nr {
    pub const YIELD: usize = 0;
    pub const EXIT: usize = 1;
    pub const PRINT: usize = 2;
    pub const SET_TLS: usize = 3;
    pub const GET_TLS: usize = 4;
    pub const SET_AFFINITY: usize = 5;
    pub const GET_AFFINITY: usize = 6;
    pub const GET_LAUNCH_STATS: usize = 7;
    pub const GET_PROCESS_COUNT: usize = 8;
    pub const LIST_PROCESS_IDS: usize = 9;
    pub const SPAWN_PROCESS: usize = 10;
    pub const GET_PROCESS_IMAGE_STATE: usize = 11;
    pub const GET_PROCESS_MAPPING_STATE: usize = 12;
    pub const VFS_MOUNT_RAMFS: usize = 13;
    pub const VFS_LIST_MOUNTS: usize = 14;
    pub const GET_POWER_STATS: usize = 15;
    pub const SET_POWER_OVERRIDE: usize = 16;
    pub const CLEAR_POWER_OVERRIDE: usize = 17;
    pub const GET_NETWORK_STATS: usize = 18;
    pub const SET_NETWORK_POLLING: usize = 19;
    pub const TERMINATE_PROCESS: usize = 20;
    pub const GET_PROCESS_LAUNCH_CONTEXT: usize = 21;
    pub const VFS_GET_MOUNT_PATH: usize = 22;
    pub const VFS_UNMOUNT: usize = 23;
    pub const VFS_GET_STATS: usize = 24;
    pub const NETWORK_RESET_STATS: usize = 25;
    pub const NETWORK_FORCE_POLL: usize = 26;
    pub const SET_CSTATE_OVERRIDE: usize = 27;
    pub const CLEAR_CSTATE_OVERRIDE: usize = 28;
    pub const CLAIM_NEXT_LAUNCH_CONTEXT: usize = 29;
    pub const ACK_LAUNCH_CONTEXT: usize = 30;
    pub const GET_LAUNCH_CONTEXT_STAGE: usize = 31;
    pub const TERMINATE_TASK: usize = 32;
    pub const GET_PROCESS_ID_BY_TASK: usize = 33;
    pub const VFS_UNMOUNT_PATH: usize = 34;
    pub const NETWORK_REINITIALIZE: usize = 35;
    pub const CONSUME_READY_LAUNCH_CONTEXT: usize = 36;
    pub const EXECUTE_READY_LAUNCH_CONTEXT: usize = 37;
    pub const FUTEX_WAIT: usize = 38;
    pub const FUTEX_WAKE: usize = 39;
    pub const UPCALL_REGISTER: usize = 40;
    pub const UPCALL_UNREGISTER: usize = 41;
    pub const UPCALL_QUERY: usize = 42;
    pub const UPCALL_CONSUME: usize = 43;
    pub const UPCALL_INJECT_VIRQ: usize = 44;
    pub const GET_ABI_INFO: usize = 45;
    pub const SET_NETWORK_BACKPRESSURE_POLICY: usize = 46;
    pub const SET_NETWORK_ALERT_THRESHOLDS: usize = 47;
    pub const GET_NETWORK_ALERT_REPORT: usize = 48;
    pub const RESOLVE_PLT: usize = 49;
    pub const VFS_OPEN: usize = 50;
    pub const VFS_READ: usize = 51;
    pub const VFS_WRITE: usize = 52;
    pub const VFS_CLOSE: usize = 53;
    pub const GET_CRASH_REPORT: usize = 54;
    pub const LIST_CRASH_EVENTS: usize = 55;
    pub const GET_CORE_PRESSURE_SNAPSHOT: usize = 56;
    pub const GET_LOTTERY_REPLAY_LATEST: usize = 57;
    pub const SET_POLICY_DRIFT_CONTROL: usize = 58;
    pub const GET_POLICY_DRIFT_CONTROL: usize = 59;
    pub const GET_POLICY_DRIFT_REASON_TEXT: usize = 60;
}

#[path = "syscalls_consts/linux_numbers.rs"]
mod linux_numbers;
pub use linux_numbers::linux_nr;

pub mod linux {
    pub const STDIN_FILENO: usize = 0;
    pub const STDOUT_FILENO: usize = 1;
    pub const STDERR_FILENO: usize = 2;
    pub const AT_NULL: usize = 0;
    pub const AT_ENTRY: usize = 9;
    pub const AT_FDCWD: isize = -100;
    pub const AT_REMOVEDIR: usize = 0x200;
    pub const AT_EMPTY_PATH: usize = 0x1000;

    pub const STACK_SIZE_8K: usize = 0x2000;
    pub const STACK_SIZE_16K: usize = 0x4000;
    pub const BRK_START: usize = 0x4000_0000;
    pub const FB_FD: usize = 9999;
    pub const INPUT_FD: usize = 9998;
    pub const PIPE_BASE_FD: usize = 30000;
    pub const MOUNT_FD_BASE: usize = 50000;
    pub const MOUNT_CTX_FD_BASE: usize = 60000;

    pub const IOV_MAX: usize = 1024;
    pub const SIGCHLD: usize = 17;
    pub const SIGKILL: i32 = 9;
    pub const SIGSTOP: i32 = 19;
    pub const SIGINFO_PAD_SIZE: usize = 29;
    pub const SIGSET_SIZE: usize = 8;
    pub const SIGALTSTACK_SIZE: usize = 24;

    pub const PR_SET_PDEATHSIG: usize = 1;
    pub const PR_GET_PDEATHSIG: usize = 2;
    pub const PR_SET_NAME: usize = 15;
    pub const PR_GET_NAME: usize = 16;
    pub const PR_NAME_MAX: usize = 16;
    pub const PIDFD_NONBLOCK: usize = open_flags::O_NONBLOCK;
    pub const MFD_HUGE_SHIFT: usize = 26;
    pub const MFD_HUGE_MASK: usize = 0x3f << MFD_HUGE_SHIFT;

    pub mod memfd_flags {
        pub const MFD_CLOEXEC: usize = 0x0001;
        pub const MFD_ALLOW_SEALING: usize = 0x0002;
        pub const MFD_HUGETLB: usize = 0x0004;
        pub const MFD_NOEXEC_SEAL: usize = 0x0008;
        pub const MFD_EXEC: usize = 0x0010;
    }

    pub const FBIOGET_VSCREENINFO: usize = 0x4600;
    pub const FBIOGET_FSCREENINFO: usize = 0x4602;
    pub const FB_VSCREENINFO_SIZE: usize = 160;
    pub const FB_FSCREENINFO_SIZE: usize = 80;

    pub const S_IFCHR: u32 = 0o020000;
    pub const S_IFDIR: u32 = 0o040000;
    pub const S_IFREG: u32 = 0o100000;
    pub const S_IFLNK: u32 = 0o120000;
    pub const S_IFMT: u32 = 0o170000;
    pub const AT_SYMLINK_NOFOLLOW: usize = 0x100;
    pub const SS_DISABLE: usize = 2;

    pub const DT_UNKNOWN: u8 = 0;
    pub const DT_DIR: u8 = 4;
    pub const DT_REG: u8 = 8;
    pub const DT_LNK: u8 = 10;

    pub const DIRENT64_BASE_SIZE: usize = 19;
    pub const STAT_BLKSIZE: i64 = 4096;
    pub const STAT_BLOCK_SIZE: i64 = 512;
    pub const POLLFD_SIZE: usize = 8;
    pub const EPOLL_EVENT_SIZE: usize = 12;

    pub const STATX_BASIC_STATS: u32 = 0x7ff;
    pub const RAMFS_MAGIC: u32 = 0x858458f6;
    pub const TMPFS_MAGIC: u32 = 0x01021994;

    pub mod epoll {
        pub const EVENTS_OFFSET: usize = 0;
        pub const DATA_OFFSET: usize = 4;
        pub const EPOLL_CTL_ADD: usize = 1;
        pub const EPOLL_CTL_DEL: usize = 2;
        pub const EPOLL_CTL_MOD: usize = 3;
    }
    pub const DEFAULT_TASK_PRIORITY: u8 = 128;

    pub mod open_flags {
        pub const O_RDONLY: usize = 0x0;
        pub const O_WRONLY: usize = 0x1;
        pub const O_RDWR: usize = 0x2;
        pub const O_ACCMODE: usize = 0x3;
        pub const O_CREAT: usize = 0x40;
        pub const O_EXCL: usize = 0x80;
        pub const O_TRUNC: usize = 0x200;
        pub const O_APPEND: usize = 0x400;
        pub const O_NONBLOCK: usize = 0x800;
        pub const O_DIRECTORY: usize = 0x10000;
        pub const O_CLOEXEC: usize = 0x80000;
        pub const O_TMPFILE: usize = 0x410000;
    }

    pub mod openat2 {
        pub const RESOLVE_NO_XDEV: usize = 0x01;
        pub const RESOLVE_NO_MAGICLINKS: usize = 0x02;
        pub const RESOLVE_NO_SYMLINKS: usize = 0x04;
        pub const RESOLVE_BENEATH: usize = 0x08;
        pub const RESOLVE_IN_ROOT: usize = 0x10;
        pub const RESOLVE_CACHED: usize = 0x20;
        pub const RESOLVE_ALLOWED_MASK: usize = RESOLVE_NO_XDEV
            | RESOLVE_NO_MAGICLINKS
            | RESOLVE_NO_SYMLINKS
            | RESOLVE_BENEATH
            | RESOLVE_IN_ROOT
            | RESOLVE_CACHED;
    }

    pub mod mountfd {
        pub const FSMOUNT_CLOEXEC: usize = 0x0000_0001;
        pub const OPEN_TREE_CLONE: usize = 0x0000_0001;
        pub const OPEN_TREE_CLOEXEC: usize = 0x0008_0000;
        pub const MOVE_MOUNT_F_EMPTY_PATH: usize = 0x0000_0004;
        pub const MOVE_MOUNT_T_EMPTY_PATH: usize = 0x0000_0040;
        pub const MOVE_MOUNT_ALLOWED_MASK: usize =
            MOVE_MOUNT_F_EMPTY_PATH | MOVE_MOUNT_T_EMPTY_PATH;
        pub const MOUNT_ATTR_RDONLY: u64 = 0x0000_0001;
    }

    pub mod clone_flags {
        pub const CLONE_VM: usize = 0x00000100;
        pub const CLONE_FS: usize = 0x00000200;
        pub const CLONE_FILES: usize = 0x00000400;
        pub const CLONE_SIGHAND: usize = 0x00000800;
        pub const CLONE_THREAD: usize = 0x00010000;
        pub const CLONE_NEWNS: usize = 0x00020000;
        pub const CLONE_SYSVSEM: usize = 0x00040000;
        pub const CLONE_SETTLS: usize = 0x00080000;
        pub const CLONE_PARENT_SETTID: usize = 0x00100000;
        pub const CLONE_CHILD_CLEARTID: usize = 0x00200000;
        pub const CLONE_CHILD_SETTID: usize = 0x01000000;
        pub const CLONE_NEWCGROUP: usize = 0x02000000;
        pub const CLONE_NEWUTS: usize = 0x04000000;
        pub const CLONE_NEWIPC: usize = 0x08000000;
        pub const CLONE_NEWUSER: usize = 0x10000000;
        pub const CLONE_NEWPID: usize = 0x20000000;
        pub const CLONE_NEWNET: usize = 0x40000000;
    }

    pub mod arch_prctl {
        pub const ARCH_SET_FS: usize = 0x1002;
        pub const ARCH_GET_FS: usize = 0x1003;
    }

    pub mod seek {
        pub const SEEK_SET: usize = 0;
        pub const SEEK_CUR: usize = 1;
        pub const SEEK_END: usize = 2;
    }

    pub mod fcntl {
        pub const F_DUPFD: usize = 0;
        pub const F_GETFD: usize = 1;
        pub const F_SETFD: usize = 2;
        pub const F_GETFL: usize = 3;
        pub const F_SETFL: usize = 4;
        pub const F_GETLK: usize = 5;
        pub const F_SETLK: usize = 6;
        pub const F_SETLKW: usize = 7;
        pub const F_SETOWN: usize = 8;
        pub const F_GETOWN: usize = 9;
        pub const F_SETSIG: usize = 10;
        pub const F_GETSIG: usize = 11;
        pub const F_DUPFD_CLOEXEC: usize = 1030;
        pub const F_OFD_GETLK: usize = 36;
        pub const F_OFD_SETLK: usize = 37;
        pub const F_OFD_SETLKW: usize = 38;
        pub const F_GETPIPE_SZ: usize = 1032;
        pub const F_SETPIPE_SZ: usize = 1031;
        pub const F_ADD_SEALS: usize = 1033;
        pub const F_GET_SEALS: usize = 1034;
        /// Lock type value: unlock (for F_GETLK response)
        pub const F_UNLCK: usize = 2;
        /// Offset of l_pid in struct flock (x86-64 ABI)
        pub const STRUCT_FLOCK_PID_OFFSET: usize = 20;
        pub const STRUCT_FLOCK_SIZE: usize = 32;
    }

    pub const O_RDWR: usize = 2;
    pub const PIPE_BUF_SIZE: usize = 65536; // 64 KiB default pipe buffer

    pub mod poll {
        pub const POLLIN: u16 = 0x0001;
        pub const POLLPRI: u16 = 0x0002;
        pub const POLLOUT: u16 = 0x0004;
        pub const POLLERR: u16 = 0x0008;
        pub const POLLHUP: u16 = 0x0010;
        pub const POLLNVAL: u16 = 0x0020;
    }

    pub mod mmap {
        pub const PROT_NONE: usize = 0;
        pub const PROT_READ: usize = 1;
        pub const PROT_WRITE: usize = 2;
        pub const PROT_EXEC: usize = 4;

        pub const MAP_SHARED: usize = 0x01;
        pub const MAP_PRIVATE: usize = 0x02;
        pub const MAP_FIXED: usize = 0x10;
        pub const MAP_ANONYMOUS: usize = 0x20;
    }

    pub mod wait {
        pub const WNOHANG: usize = 1;
        pub const WUNTRACED: usize = 2;
        pub const WSTOPPED: usize = 2;
        pub const WEXITED: usize = 4;
        pub const WCONTINUED: usize = 8;
        pub const WNOWAIT: usize = 0x0100_0000;
    }

    pub mod futex {
        pub const FUTEX_WAIT: usize = 0;
        pub const FUTEX_WAKE: usize = 1;
        pub const FUTEX_FD: usize = 2;
        pub const FUTEX_REQUEUE: usize = 3;
        pub const FUTEX_CMP_REQUEUE: usize = 4;
        pub const FUTEX_CMD_MASK: usize = 0x7f;
    }
}

#[path = "syscalls_consts/counters.rs"]
mod counters;
pub use counters::*;

pub const MAX_PRINT_LEN: usize = crate::generated_consts::IPC_MSG_SIZE_LIMIT;
pub const USER_CSTRING_MAX_LEN: usize = 4096;
pub const MOUNT_RECORD_WORDS: usize = 3;
pub const PROCESS_LIST_LIMIT: usize = 64;
pub const PROCESS_LAUNCH_CTX_WORDS: usize = 8;
pub const UPCALL_QUERY_WORDS: usize = 4;
pub const UPCALL_DELIVERY_WORDS: usize = 5;
pub const SYSCALL_ABI_INFO_WORDS: usize = 7;
pub const SYSCALL_ABI_MAGIC: usize = 0x48594241; // HYBA
pub const SYSCALL_ABI_VERSION_MAJOR: usize = 1;
pub const SYSCALL_ABI_VERSION_MINOR: usize = 0;
pub const SYSCALL_ABI_VERSION_PATCH: usize = 0;
pub const SYSCALL_ABI_MIN_COMPAT_MAJOR: usize = 1;
pub const SYSCALL_ABI_FLAG_STABLE: usize = 1 << 0;
pub const CRASH_REPORT_WORDS: usize = 10;
pub const CRASH_EVENT_WORDS: usize = 8;
pub const CORE_PRESSURE_SNAPSHOT_WORDS: usize = 18;
pub const LOTTERY_REPLAY_LATEST_WORDS: usize = 5;

#[inline(always)]
pub fn required_bytes(words: usize) -> usize {
    words * core::mem::size_of::<usize>()
}

#[inline(always)]
pub fn write_launch_context_words(
    out: &mut [usize],
    process_id: usize,
    task_id: usize,
    entry: usize,
    image_pages: usize,
    image_segments: usize,
    mapped_regions: usize,
    mapped_pages: usize,
    cr3: usize,
) {
    out[0] = process_id;
    out[1] = task_id;
    out[2] = entry;
    out[3] = image_pages;
    out[4] = image_segments;
    out[5] = mapped_regions;
    out[6] = mapped_pages;
    out[7] = cr3;
}
