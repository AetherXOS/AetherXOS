/// Global Process Registry
///
/// All live processes are stored here, indexed by `ProcessId`.
/// The registry is the single source of truth for process lifetime:
///   * Processes are inserted on `fork` / initialization.
///   * Processes are removed on `exit` / `wait` collection.
///   * Every lookup returns an `Arc<Process>` so the struct survives
///     a concurrent removal without use-after-free.
///
/// # Locking
/// A single `IrqSafeMutex<BTreeMap>` guards the table.
/// Callers must NOT hold the registry lock while acquiring per-Process locks.
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;

use crate::interfaces::task::ProcessId;
use crate::kernel::process::Process;
use crate::kernel::sync::IrqSafeMutex;

type Registry = IrqSafeMutex<BTreeMap<ProcessId, Arc<Process>>>;

lazy_static! {
    static ref PROCESS_TABLE: Registry = IrqSafeMutex::new(BTreeMap::new());
}

/// Total number of process IDs ever allocated (monotonic).
static PROCESS_COUNT: AtomicUsize = AtomicUsize::new(0);

// ── Public API ────────────────────────────────────────────────────────────────

/// Insert a new process.  The caller owns `process`; it is moved into an `Arc`.
pub fn register_process(process: Process) -> Arc<Process> {
    let pid = process.id;
    let arc = Arc::new(process);
    PROCESS_TABLE.lock().insert(pid, arc.clone());
    PROCESS_COUNT.fetch_add(1, Ordering::Relaxed);
    arc
}

/// Look up a process by ID.  Returns `None` if the process has exited.
pub fn get_process(pid: ProcessId) -> Option<Arc<Process>> {
    PROCESS_TABLE.lock().get(&pid).cloned()
}

/// Remove a process from the registry (called on process exit / reap).
/// Returns the `Arc` so the caller can drain FDs or wait for threads.
pub fn unregister_process(pid: ProcessId) -> Option<Arc<Process>> {
    PROCESS_TABLE.lock().remove(&pid)
}

/// Current number of live processes in the registry.
pub fn process_count() -> usize {
    PROCESS_TABLE.lock().len()
}

/// Total number of process IDs ever allocated (includes dead processes).
pub fn total_process_count() -> usize {
    PROCESS_COUNT.load(Ordering::Relaxed)
}

/// Collect PIDs of all live processes.
pub fn all_pids() -> alloc::vec::Vec<ProcessId> {
    PROCESS_TABLE.lock().keys().copied().collect()
}

/// Collect PIDs of all threads that belong to process `pid`.
/// Returns an empty vec if the process is not found.
pub fn threads_of(pid: ProcessId) -> alloc::vec::Vec<crate::interfaces::task::TaskId> {
    get_process(pid)
        .map(|p| p.threads.lock().clone())
        .unwrap_or_default()
}
