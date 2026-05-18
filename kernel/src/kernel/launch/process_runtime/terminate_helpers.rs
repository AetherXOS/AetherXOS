use super::*;
use alloc::string::String;
use core::fmt::Write;
use crate::klog_info;

#[cfg(feature = "process_abstraction")]
pub(super) fn log_process_exit_requested(process_id: ProcessId, status: i32) {
    crate::klog_info!(
        "process exit requested: pid={} status={}",
        process_id.0,
        status,
    );
}

#[cfg(feature = "process_abstraction")]
pub(super) fn log_process_exit_accepted(process_id: ProcessId, task_id: TaskId, status: i32) {
    crate::klog_info!(
        "process exit accepted: pid={} tid={} status={} ",
        process_id.0,
        task_id.0,
        status,
    );
}

#[cfg(feature = "process_abstraction")]
pub(super) fn log_deferred_fini_reports(process_id: ProcessId, process_arc: &alloc::sync::Arc<crate::kernel::process::Process>, shared_object_fini: &[crate::kernel::dynamic_linker::api::DeferredSharedObjectFiniReport]) {
    if shared_object_fini.is_empty() {
        return;
    }

    let pending_calls = shared_object_fini
        .iter()
        .map(|report| report.fini_calls.len())
        .sum::<usize>();
    for report in shared_object_fini {
        process_arc.append_deferred_fini_calls(&report.fini_calls);
    }
    klog_info!(
        "process exit: pid={} deferred_shared_object_fini_reports={} total_calls={}",
        process_id.0,
        shared_object_fini.len(),
        pending_calls,
    );
}

#[cfg(feature = "process_abstraction")]
pub(super) fn log_runtime_contract_exit(process_id: ProcessId, runtime_contract: &crate::kernel::process::RuntimeContract, status: i32) {
    if !runtime_contract.fini_calls.is_empty() {
        let mut fini_preview = String::new();
        for (idx, addr) in runtime_contract.fini_calls.iter().take(4).enumerate() {
            if idx != 0 {
                fini_preview.push(',');
            }
            let _ = Write::write_fmt(&mut fini_preview, format_args!("{:#x}", addr));
        }
        klog_info!(
            "process exit: pid={} exec='{}' status={} pending_fini_hooks={} fini_preview=[{}] vdso={:#x}",
            process_id.0,
            runtime_contract.exec_path.as_str(),
            status,
            runtime_contract.fini_calls.len(),
            fini_preview.as_str(),
            runtime_contract.vdso_base,
        );
    }

    if runtime_contract.runtime_fini_entry != 0 {
        RUNTIME_FINI_TRAMPOLINES_SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        RUNTIME_FINI_EXECUTION_DEFERRED.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        klog_info!(
            "process exit: pid={} exec='{}' runtime_fini_entry={:#x} pending_fini_hooks={} execution=deferred",
            process_id.0,
            runtime_contract.exec_path.as_str(),
            runtime_contract.runtime_fini_entry,
            runtime_contract.fini_calls.len(),
        );
    }
}

#[cfg(feature = "process_abstraction")]
pub(super) fn cleanup_task_on_all_cpus(task_id: TaskId) {
    let cpus = crate::hal::smp::CPUS.lock();
    for cpu in cpus.iter() {
        let mut scheduler = cpu.scheduler.lock();
        scheduler.remove_task(task_id);
    }
}
