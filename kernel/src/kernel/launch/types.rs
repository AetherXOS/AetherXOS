use alloc::vec::Vec;
use crate::interfaces::task::{ProcessId, TaskId};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    LoaderFailed,
    SchedulerUnavailable,
    InvalidSpawnRequest,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone)]
pub enum BootImageRecord {
    Owned(Vec<u8>),
    BorrowedStatic(&'static [u8]),
    OwnedAligned(Vec<u64>, usize),
}

#[cfg(feature = "process_abstraction")]
impl BootImageRecord {
    pub fn as_slice(&self) -> &[u8] {
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

    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchStage {
    Pending,
    Claimed,
    Ready,
}

#[cfg(feature = "process_abstraction")]
impl LaunchStage {
    pub fn as_usize(&self) -> usize {
        match self {
            Self::Pending => 0,
            Self::Claimed => 1,
            Self::Ready => 2,
        }
    }
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone, Copy)]
pub struct LaunchRegistrySnapshotEntry {
    pub process_id: ProcessId,
    pub task_id: TaskId,
    pub stage: usize,
    pub image_pages: usize,
    pub mapped_pages: usize,
}

#[cfg(feature = "process_abstraction")]
impl Default for LaunchRegistrySnapshotEntry {
    fn default() -> Self {
        Self {
            process_id: ProcessId(0),
            task_id: TaskId(0),
            stage: 0,
            image_pages: 0,
            mapped_pages: 0,
        }
    }
}
#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone)]
pub struct PreparedBootstrap {
    pub boot_image: BootImageRecord,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone)]
pub struct AlignedStaticDispatch {
    pub prepared_bootstrap: PreparedBootstrap,
}
