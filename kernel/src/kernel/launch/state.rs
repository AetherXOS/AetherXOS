use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::kernel::sync::IrqSafeMutex;
use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::kernel::process::Process;
use super::types::*;

pub static SPAWN_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static SPAWN_SUCCESS: AtomicU64 = AtomicU64::new(0);
pub static SPAWN_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static ENQUEUE_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static VALIDATION_FAILURES: AtomicU64 = AtomicU64::new(0);

pub static TERMINATE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static TERMINATE_SUCCESS: AtomicU64 = AtomicU64::new(0);
pub static TERMINATE_FAILURES: AtomicU64 = AtomicU64::new(0);

pub static CLAIM_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static CLAIM_SUCCESS: AtomicU64 = AtomicU64::new(0);
pub static CLAIM_FAILURES: AtomicU64 = AtomicU64::new(0);

pub static HANDOFF_ACK_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static HANDOFF_ACK_SUCCESS: AtomicU64 = AtomicU64::new(0);
pub static HANDOFF_ACK_FAILURES: AtomicU64 = AtomicU64::new(0);

pub static HANDOFF_CONSUME_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static HANDOFF_CONSUME_SUCCESS: AtomicU64 = AtomicU64::new(0);
pub static HANDOFF_CONSUME_FAILURES: AtomicU64 = AtomicU64::new(0);

pub static HANDOFF_EXECUTE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static HANDOFF_EXECUTE_SUCCESS: AtomicU64 = AtomicU64::new(0);
pub static HANDOFF_EXECUTE_FAILURES: AtomicU64 = AtomicU64::new(0);

pub static TERMINATE_BY_TASK_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static TERMINATE_BY_TASK_SUCCESS: AtomicU64 = AtomicU64::new(0);
pub static TERMINATE_BY_TASK_FAILURES: AtomicU64 = AtomicU64::new(0);

pub static STALE_SCAN_CALLS: AtomicU64 = AtomicU64::new(0);
pub static STALE_RECYCLED_ENTRIES: AtomicU64 = AtomicU64::new(0);
pub static STALE_CLAIM_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub static STALE_READY_TIMEOUTS: AtomicU64 = AtomicU64::new(0);

pub static RUNTIME_FINI_TRAMPOLINES_SEEN: AtomicU64 = AtomicU64::new(0);
pub static RUNTIME_FINI_EXECUTION_DEFERRED: AtomicU64 = AtomicU64::new(0);

pub static LAST_TASK_ID: AtomicUsize = AtomicUsize::new(0);

pub static HANDOFF_EPOCH: AtomicU64 = AtomicU64::new(1);

#[cfg(feature = "process_abstraction")]
pub struct LaunchRegistryEntry {
    pub process_id: crate::interfaces::task::ProcessId,
    pub task_id: crate::interfaces::task::TaskId,
    pub process: Arc<Process>,
    pub boot_image: BootImageRecord,
    pub stage: LaunchStage,
    pub stage_epoch: u64,
}

#[cfg(feature = "process_abstraction")]
lazy_static::lazy_static! {
    pub static ref PROCESS_REGISTRY: IrqSafeMutex<Vec<LaunchRegistryEntry>> = IrqSafeMutex::new(Vec::new());
}

pub fn next_handoff_epoch() -> u64 {
    HANDOFF_EPOCH.fetch_add(1, Ordering::Relaxed)
}
