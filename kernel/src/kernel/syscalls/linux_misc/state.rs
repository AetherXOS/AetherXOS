use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, AtomicU32};
use lazy_static::lazy_static;
use crate::kernel::sync::IrqSafeMutex;
use super::types::*;

lazy_static! {
    pub static ref TIMERFD_STATE_BY_FD: IrqSafeMutex<BTreeMap<u32, TimerfdRuntimeState>> =
        IrqSafeMutex::new(BTreeMap::new());
    pub static ref IO_URING_IDS: IrqSafeMutex<BTreeSet<u32>> =
        IrqSafeMutex::new(BTreeSet::new());
    pub static ref LANDLOCK_RULESET_IDS: IrqSafeMutex<BTreeSet<u32>> =
        IrqSafeMutex::new(BTreeSet::new());
    pub static ref BPF_MAP_IDS: IrqSafeMutex<BTreeSet<u32>> =
        IrqSafeMutex::new(BTreeSet::new());
    pub static ref FANOTIFY_MARKS_BY_FD: IrqSafeMutex<BTreeMap<u32, Vec<FanotifyMarkState>>> =
        IrqSafeMutex::new(BTreeMap::new());
    pub static ref EVENTFD_STATE_BY_FD: IrqSafeMutex<BTreeMap<u32, u64>> =
        IrqSafeMutex::new(BTreeMap::new());
    pub static ref MEMFD_NAME_BY_FD: IrqSafeMutex<BTreeMap<u32, String>> =
        IrqSafeMutex::new(BTreeMap::new());
    pub static ref INOTIFY_WATCHES_BY_FD: IrqSafeMutex<BTreeMap<u32, Vec<InotifyWatchState>>> =
        IrqSafeMutex::new(BTreeMap::new());
    pub static ref SIGNALFD_MASK_BY_FD: IrqSafeMutex<BTreeMap<u32, u64>> =
        IrqSafeMutex::new(BTreeMap::new());
}

pub static NEXT_IO_URING_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_LANDLOCK_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_BPF_MAP_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_FANOTIFY_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_EVENTFD_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_TIMERFD_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_MEMFD_SYNTH_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_INOTIFY_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_INOTIFY_WD: AtomicI32 = AtomicI32::new(1);
pub static NEXT_SIGNALFD_ID: AtomicU32 = AtomicU32::new(1);

pub const IO_URING_FD_BASE: usize = 700_000;
pub const LANDLOCK_FD_BASE: usize = 710_000;
pub const BPF_FD_BASE: usize = 720_000;
pub const FANOTIFY_FD_BASE: usize = 730_000;
pub const EVENTFD_FD_BASE: usize = 740_000;
pub const TIMERFD_FD_BASE: usize = 750_000;
pub const MEMFD_FD_BASE: usize = 760_000;
pub const INOTIFY_FD_BASE: usize = 770_000;
pub const SIGNALFD_FD_BASE: usize = 780_000;
