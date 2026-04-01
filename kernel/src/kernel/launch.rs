use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::interfaces::task::{ProcessId, TaskId};
use crate::interfaces::{Scheduler, TaskState};
use crate::kernel::cpu_local::CpuLocal;
use crate::kernel::sync::IrqSafeMutex;

#[cfg(feature = "process_abstraction")]
use crate::kernel::process::Process;

#[derive(Debug, Clone, Copy)]
pub struct LaunchStats {
    pub spawn_attempts: u64,
    pub spawn_success: u64,
    pub spawn_failures: u64,
    pub enqueue_failures: u64,
    pub validation_failures: u64,
    pub terminate_attempts: u64,
    pub terminate_success: u64,
    pub terminate_failures: u64,
    pub claim_attempts: u64,
    pub claim_success: u64,
    pub claim_failures: u64,
    pub handoff_ack_attempts: u64,
    pub handoff_ack_success: u64,
    pub handoff_ack_failures: u64,
    pub handoff_consume_attempts: u64,
    pub handoff_consume_success: u64,
    pub handoff_consume_failures: u64,
    pub handoff_execute_attempts: u64,
    pub handoff_execute_success: u64,
    pub handoff_execute_failures: u64,
    pub terminate_by_task_attempts: u64,
    pub terminate_by_task_success: u64,
    pub terminate_by_task_failures: u64,
    pub stale_scan_calls: u64,
    pub stale_recycled_entries: u64,
    pub stale_claim_timeouts: u64,
    pub stale_ready_timeouts: u64,
    pub runtime_fini_trampolines_seen: u64,
    pub runtime_fini_execution_deferred: u64,
    pub registered_processes: usize,
    pub last_task_id: TaskId,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone, Copy)]
pub struct LaunchContext {
    pub process_id: ProcessId,
    pub task_id: TaskId,
    pub entry: usize,
    pub image_pages: usize,
    pub image_segments: usize,
    pub exec_generation: u64,
    pub mapped_regions: usize,
    pub mapped_pages: usize,
    pub cr3: usize,
}

#[cfg(feature = "process_abstraction")]
pub use process_runtime::LaunchRegistrySnapshotEntry;
#[cfg(feature = "process_abstraction")]
pub use process_runtime::launch_registry_snapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    LoaderFailed,
    SchedulerUnavailable,
    InvalidSpawnRequest,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone)]
enum BootImageRecord {
    Owned(Vec<u8>),
    BorrowedStatic(&'static [u8]),
    OwnedAligned(Vec<u64>, usize),
}

#[cfg(feature = "process_abstraction")]
impl BootImageRecord {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes.as_slice(),
            Self::BorrowedStatic(bytes) => bytes,
            Self::OwnedAligned(words, len) => {
                let byte_len = words.len() * core::mem::size_of::<u64>();
                let bytes = unsafe {
                    core::slice::from_raw_parts(words.as_ptr() as *const u8, byte_len)
                };
                &bytes[..*len]
            }
        }
    }

    fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

static SPAWN_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static SPAWN_SUCCESS: AtomicU64 = AtomicU64::new(0);
static SPAWN_FAILURES: AtomicU64 = AtomicU64::new(0);
static ENQUEUE_FAILURES: AtomicU64 = AtomicU64::new(0);
static VALIDATION_FAILURES: AtomicU64 = AtomicU64::new(0);
static TERMINATE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static TERMINATE_SUCCESS: AtomicU64 = AtomicU64::new(0);
static TERMINATE_FAILURES: AtomicU64 = AtomicU64::new(0);
static CLAIM_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static CLAIM_SUCCESS: AtomicU64 = AtomicU64::new(0);
static CLAIM_FAILURES: AtomicU64 = AtomicU64::new(0);
static HANDOFF_ACK_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static HANDOFF_ACK_SUCCESS: AtomicU64 = AtomicU64::new(0);
static HANDOFF_ACK_FAILURES: AtomicU64 = AtomicU64::new(0);
static HANDOFF_CONSUME_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static HANDOFF_CONSUME_SUCCESS: AtomicU64 = AtomicU64::new(0);
static HANDOFF_CONSUME_FAILURES: AtomicU64 = AtomicU64::new(0);
static HANDOFF_EXECUTE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static HANDOFF_EXECUTE_SUCCESS: AtomicU64 = AtomicU64::new(0);
static HANDOFF_EXECUTE_FAILURES: AtomicU64 = AtomicU64::new(0);
static TERMINATE_BY_TASK_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static TERMINATE_BY_TASK_SUCCESS: AtomicU64 = AtomicU64::new(0);
static TERMINATE_BY_TASK_FAILURES: AtomicU64 = AtomicU64::new(0);
static STALE_SCAN_CALLS: AtomicU64 = AtomicU64::new(0);
static STALE_RECYCLED_ENTRIES: AtomicU64 = AtomicU64::new(0);
static STALE_CLAIM_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static STALE_READY_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static RUNTIME_FINI_TRAMPOLINES_SEEN: AtomicU64 = AtomicU64::new(0);
static RUNTIME_FINI_EXECUTION_DEFERRED: AtomicU64 = AtomicU64::new(0);
static HANDOFF_EPOCH: AtomicU64 = AtomicU64::new(0);
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);
static LAST_TASK_ID: AtomicUsize = AtomicUsize::new(0);
const ROBUST_LIST_MAX_NODES: usize = 64;
const ROBUST_LIST_HEAD_WORDS: usize = 3;
const FUTEX_OWNER_DIED_BIT: u32 = 0x4000_0000;
const FUTEX_WAITERS_BIT: u32 = 0x8000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LaunchStage {
    Pending = 0,
    Claimed = 1,
    Ready = 2,
}

impl LaunchStage {
    #[inline(always)]
    const fn as_usize(self) -> usize {
        self as usize
    }
}

#[cfg(feature = "process_abstraction")]
struct RegistryEntry {
    process_id: ProcessId,
    process: Arc<Process>,
    task_id: TaskId,
    boot_image: BootImageRecord,
    stage: LaunchStage,
    stage_epoch: u64,
}

#[cfg(feature = "process_abstraction")]
static PROCESS_REGISTRY: IrqSafeMutex<Vec<RegistryEntry>> = IrqSafeMutex::new(Vec::new());

#[inline(always)]
fn allocate_task_id() -> TaskId {
    TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
}

#[inline(always)]
fn next_handoff_epoch() -> u64 {
    HANDOFF_EPOCH
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1)
}

fn finalize_task_user_exit_state(task_id: TaskId) {
    finalize_task_robust_futexes(task_id);

    let Some(task_arc) = crate::kernel::task::get_task(task_id) else {
        crate::kernel::syscalls::clear_robust_list_for_tid(task_id.0);
        return;
    };

    let clear_child_tid = {
        let mut task = task_arc.lock();
        let ptr = task.clear_child_tid;
        task.clear_child_tid = 0;
        task.state = TaskState::Terminated;
        ptr
    };

    if clear_child_tid != 0 {
        let _ = crate::kernel::syscalls::with_user_write_words(
            clear_child_tid,
            core::mem::size_of::<usize>(),
            1,
            |out| {
                out[0] = 0;
            },
        );
        let _ = crate::kernel::syscalls::sys_futex_wake(clear_child_tid, 1, 0);
    }

    crate::kernel::syscalls::clear_robust_list_for_tid(task_id.0);
}

fn finalize_task_robust_futexes(task_id: TaskId) {
    let Some((head_ptr, head_len)) = crate::kernel::syscalls::robust_list_for_tid(task_id.0) else {
        return;
    };
    if head_ptr == 0 || head_len != crate::generated_consts::LINUX_ROBUST_LIST_HEAD_SIZE {
        return;
    }

    let head = match crate::kernel::syscalls::with_user_read_bytes(
        head_ptr,
        core::mem::size_of::<usize>() * ROBUST_LIST_HEAD_WORDS,
        |src| {
            let word_bytes = core::mem::size_of::<usize>();
            let mut words = [0usize; ROBUST_LIST_HEAD_WORDS];
            for (idx, slot) in words.iter_mut().enumerate() {
                let start = idx * word_bytes;
                let end = start + word_bytes;
                let mut raw = [0u8; core::mem::size_of::<usize>()];
                raw.copy_from_slice(&src[start..end]);
                *slot = usize::from_ne_bytes(raw);
            }
            words
        },
    ) {
        Ok(words) => words,
        Err(_) => return,
    };

    let list_head = head_ptr;
    let mut next = head[0];
    let futex_offset = head[1] as isize;
    let op_pending = head[2];

    if op_pending != 0 {
        mark_robust_futex_owner_died(op_pending, futex_offset);
    }

    let mut scanned = 0usize;
    while next != 0 && next != list_head && scanned < ROBUST_LIST_MAX_NODES {
        let node = next;
        let next_node = match crate::kernel::syscalls::with_user_read_bytes(
            node,
            core::mem::size_of::<usize>(),
            |src| {
                let mut raw = [0u8; core::mem::size_of::<usize>()];
                raw.copy_from_slice(src);
                usize::from_ne_bytes(raw)
            },
        ) {
            Ok(value) => value,
            Err(_) => break,
        };

        mark_robust_futex_owner_died(node, futex_offset);
        next = next_node;
        scanned = scanned.saturating_add(1);
    }
}

fn mark_robust_futex_owner_died(node_ptr: usize, futex_offset: isize) {
    let Some(futex_addr) = node_ptr.checked_add_signed(futex_offset) else {
        return;
    };
    if (futex_addr & (core::mem::size_of::<u32>() - 1)) != 0 {
        return;
    }

    let old_word = match crate::kernel::syscalls::with_user_read_bytes(
        futex_addr,
        core::mem::size_of::<u32>(),
        |src| {
            let mut raw = [0u8; core::mem::size_of::<u32>()];
            raw.copy_from_slice(src);
            u32::from_ne_bytes(raw)
        },
    ) {
        Ok(word) => word,
        Err(_) => return,
    };

    let new_word = (old_word & FUTEX_WAITERS_BIT) | FUTEX_OWNER_DIED_BIT;
    let _ = crate::kernel::syscalls::with_user_write_bytes(
        futex_addr,
        core::mem::size_of::<u32>(),
        |dst| dst.copy_from_slice(&new_word.to_ne_bytes()),
    );
    let _ = crate::kernel::syscalls::sys_futex_wake(futex_addr, 1, 0);
}


#[cfg(feature = "process_abstraction")]
#[path = "launch/process_runtime.rs"]
mod process_runtime;
#[cfg(feature = "process_abstraction")]
pub use process_runtime::{
    acknowledge_launch_context,
    acknowledge_launch_context_typed,
    claim_next_launch_context,
    clone_process_from_registered_image,
    consume_ready_launch_context,
    current_process_arc,
    execute_ready_launch_context_on_current_cpu,
    launch_context_stage,
    launch_context_stage_typed,
    process_arc_by_id,
    process_boot_image,
    process_boot_image_typed,
    process_count,
    process_id_by_task,
    process_ids_snapshot,
    process_image_state,
    process_launch_context,
    process_launch_context_typed,
    process_mapping_state,
    process_register_mapping_typed,
    spawn_bootstrap_from_aligned_static_image,
    spawn_bootstrap_from_image,
    spawn_bootstrap_from_static_image,
    terminate_process,
    terminate_process_with_status,
    terminate_task,
};
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub use process_runtime::process_materialize_mapping_typed;
#[cfg(all(feature = "process_abstraction", feature = "posix_mman"))]
pub use process_runtime::refresh_all_linux_runtime_vvar;

pub fn stats() -> LaunchStats {
    #[cfg(feature = "process_abstraction")]
    let registered_processes = process_count();
    #[cfg(not(feature = "process_abstraction"))]
    let registered_processes = 0;

    LaunchStats {
        spawn_attempts: SPAWN_ATTEMPTS.load(Ordering::Relaxed),
        spawn_success: SPAWN_SUCCESS.load(Ordering::Relaxed),
        spawn_failures: SPAWN_FAILURES.load(Ordering::Relaxed),
        enqueue_failures: ENQUEUE_FAILURES.load(Ordering::Relaxed),
        validation_failures: VALIDATION_FAILURES.load(Ordering::Relaxed),
        terminate_attempts: TERMINATE_ATTEMPTS.load(Ordering::Relaxed),
        terminate_success: TERMINATE_SUCCESS.load(Ordering::Relaxed),
        terminate_failures: TERMINATE_FAILURES.load(Ordering::Relaxed),
        claim_attempts: CLAIM_ATTEMPTS.load(Ordering::Relaxed),
        claim_success: CLAIM_SUCCESS.load(Ordering::Relaxed),
        claim_failures: CLAIM_FAILURES.load(Ordering::Relaxed),
        handoff_ack_attempts: HANDOFF_ACK_ATTEMPTS.load(Ordering::Relaxed),
        handoff_ack_success: HANDOFF_ACK_SUCCESS.load(Ordering::Relaxed),
        handoff_ack_failures: HANDOFF_ACK_FAILURES.load(Ordering::Relaxed),
        handoff_consume_attempts: HANDOFF_CONSUME_ATTEMPTS.load(Ordering::Relaxed),
        handoff_consume_success: HANDOFF_CONSUME_SUCCESS.load(Ordering::Relaxed),
        handoff_consume_failures: HANDOFF_CONSUME_FAILURES.load(Ordering::Relaxed),
        handoff_execute_attempts: HANDOFF_EXECUTE_ATTEMPTS.load(Ordering::Relaxed),
        handoff_execute_success: HANDOFF_EXECUTE_SUCCESS.load(Ordering::Relaxed),
        handoff_execute_failures: HANDOFF_EXECUTE_FAILURES.load(Ordering::Relaxed),
        terminate_by_task_attempts: TERMINATE_BY_TASK_ATTEMPTS.load(Ordering::Relaxed),
        terminate_by_task_success: TERMINATE_BY_TASK_SUCCESS.load(Ordering::Relaxed),
        terminate_by_task_failures: TERMINATE_BY_TASK_FAILURES.load(Ordering::Relaxed),
        stale_scan_calls: STALE_SCAN_CALLS.load(Ordering::Relaxed),
        stale_recycled_entries: STALE_RECYCLED_ENTRIES.load(Ordering::Relaxed),
        stale_claim_timeouts: STALE_CLAIM_TIMEOUTS.load(Ordering::Relaxed),
        stale_ready_timeouts: STALE_READY_TIMEOUTS.load(Ordering::Relaxed),
        runtime_fini_trampolines_seen: RUNTIME_FINI_TRAMPOLINES_SEEN.load(Ordering::Relaxed),
        runtime_fini_execution_deferred: RUNTIME_FINI_EXECUTION_DEFERRED.load(Ordering::Relaxed),
        registered_processes,
        last_task_id: TaskId(LAST_TASK_ID.load(Ordering::Relaxed)),
    }
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod launch_tests;
