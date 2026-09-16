//! # Safety
//!
//! All `unsafe` blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
use super::*;
use alloc::sync::Arc;
use crate::kernel::process::Process;
use crate::kernel::cpu_local::CpuLocal;
use core::sync::atomic::Ordering;

#[cfg(feature = "process_abstraction")]
pub(super) fn find_process_entry(process_id: ProcessId) -> Option<LaunchRegistryEntry> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .cloned()
}

#[cfg(feature = "process_abstraction")]
pub(super) fn log_query_miss(function_name: &str, process_id: ProcessId) {
    crate::klog_warn!("launch query miss: {} pid={}", function_name, process_id.0);
}

#[cfg(feature = "process_abstraction")]
pub(super) fn process_arc_from_entry(entry: &LaunchRegistryEntry) -> Arc<Process> {
    entry.process.clone()
}

#[cfg(feature = "process_abstraction")]
pub(super) fn current_process_id() -> Option<ProcessId> {
    unsafe {
        CpuLocal::try_get().map(|cpu| ProcessId(cpu.current_process_id.load(Ordering::Relaxed)))
    }
}

