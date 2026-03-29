use alloc::collections::{BTreeMap, BTreeSet};
use core::sync::atomic::AtomicU32;
use spin::Mutex;

pub(super) static NEXT_LINUX_TIMER_ID: AtomicU32 = AtomicU32::new(1);
pub(super) static NEXT_MEMFD_ID: AtomicU32 = AtomicU32::new(1);
pub(super) static NEXT_LANDLOCK_RULESET_ID: AtomicU32 = AtomicU32::new(1);
pub(super) static NEXT_IO_URING_FD: AtomicU32 = AtomicU32::new(1);
pub(super) static NEXT_USERFAULTFD_ID: AtomicU32 = AtomicU32::new(1);
pub(super) static NEXT_LEGACY_MODULE_ID: AtomicU32 = AtomicU32::new(1);
pub(super) static NEXT_BPF_MAP_ID: AtomicU32 = AtomicU32::new(1);

pub(super) static LINUX_TIMER_IDS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
pub(super) static LINUX_LANDLOCK_RULESETS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
pub(super) static LINUX_IO_URING_IDS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
pub(super) static LINUX_USERFAULTFD_IDS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
pub(super) static LEGACY_MODULES: Mutex<BTreeMap<alloc::string::String, usize>> =
    Mutex::new(BTreeMap::new());
pub(super) static BPF_MAP_IDS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

pub(super) static REBOOT_LAST_CMD: AtomicU32 = AtomicU32::new(0);
pub(super) static IOPL_LEVEL: AtomicU32 = AtomicU32::new(0);
pub(super) static IOPERM_ENABLED_RANGES: Mutex<BTreeSet<(usize, usize)>> =
    Mutex::new(BTreeSet::new());
pub(super) static KEXEC_STAGED_STATE: Mutex<Option<(u32, u32, usize)>> = Mutex::new(None);
