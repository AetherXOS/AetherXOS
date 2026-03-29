use super::*;
use crate::klog_info;
#[cfg(target_os = "none")]
use crate::interfaces::HardwareAbstraction;
#[path = "process_runtime_support.rs"]
mod process_runtime_support;
#[path = "process_runtime_wrappers.rs"]
mod process_runtime_wrappers;
#[cfg(feature = "process_abstraction")]
#[path = "process_runtime_bootstrap_dispatch.rs"]
mod process_runtime_bootstrap_dispatch;
#[cfg(feature = "process_abstraction")]
pub use process_runtime_wrappers::{
    acknowledge_launch_context, launch_context_stage, process_boot_image, process_launch_context,
    terminate_process,
};
#[cfg(feature = "process_abstraction")]
pub use process_runtime_bootstrap_dispatch::{
    clone_process_from_registered_image, spawn_bootstrap_from_aligned_static_image,
};
#[cfg(feature = "process_abstraction")]
use process_runtime_support::{
    build_context, recycle_stale_handoffs, register_process, register_process_with_task_image,
};
#[cfg(feature = "process_abstraction")]
use process_runtime_bootstrap_dispatch::{
    aligned_static_boot_image_record, record_launch_image_preview,
};
#[cfg(feature = "paging_enable")]
use process_runtime_support::current_cr3_phys;
#[cfg(feature = "process_abstraction")]
const PROCESS_PREPARE_ERROR_BASE: u64 = 0x100;
#[cfg(feature = "process_abstraction")]
const PROCESS_PREPARE_ERROR_PROCESS_BIND_FAILED: u64 = 0x200;
#[cfg(feature = "process_abstraction")]
const PROCESS_PREPARE_ERROR_MAPPING_BIND_FAILED: u64 = 0x201;
#[cfg(feature = "process_abstraction")]
const PROCESS_PREPARE_ERROR_PAGING_APPLY_FAILED: u64 = 0x202;
#[cfg(feature = "process_abstraction")]
const PROCESS_PREPARE_ERROR_SEGMENT_MATERIALIZATION_FAILED: u64 = 0x203;
#[cfg(feature = "process_abstraction")]
const PROCESS_LOOKUP_NOT_FOUND: &str = "not found";
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
const PROCESS_MATERIALIZE_FAILED: &str = "materialize failed";
#[cfg(feature = "process_abstraction")]
fn process_prepare_error_code(err: crate::kernel::module_loader::ProcessPrepareError) -> u64 {
    match err {
        crate::kernel::module_loader::ProcessPrepareError::Loader(loader) => {
            PROCESS_PREPARE_ERROR_BASE + loader as u64
        }
        crate::kernel::module_loader::ProcessPrepareError::ProcessBindFailed => {
            PROCESS_PREPARE_ERROR_PROCESS_BIND_FAILED
        }
        crate::kernel::module_loader::ProcessPrepareError::MappingBindFailed => {
            PROCESS_PREPARE_ERROR_MAPPING_BIND_FAILED
        }
        crate::kernel::module_loader::ProcessPrepareError::PagingApplyFailed => {
            PROCESS_PREPARE_ERROR_PAGING_APPLY_FAILED
        }
        crate::kernel::module_loader::ProcessPrepareError::SegmentMaterializationFailed => {
            PROCESS_PREPARE_ERROR_SEGMENT_MATERIALIZATION_FAILED
        }
    }
}
#[cfg(feature = "process_abstraction")]
#[cfg(feature = "paging_enable")]
fn preflight_bootstrap_image(
    process_name: &[u8],
    image: &[u8],
) -> Result<(), LaunchError> {
    if let Err(err) = crate::kernel::module_loader::preflight_module_image(image) {
        crate::kernel::debug_trace::record_optional(
            "launch.bootstrap",
            "preflight_failed",
            Some(err as u64),
            false,
        );
        crate::klog_warn!(
            "[LAUNCH] preflight rejected bootstrap image name='{}' bytes={} error={:?}",
            alloc::string::String::from_utf8_lossy(process_name),
            image.len(),
            err,
        );
        VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        SPAWN_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(LaunchError::LoaderFailed);
    }
    Ok(())
}
#[cfg(feature = "process_abstraction")]
#[cfg(not(feature = "paging_enable"))]
fn preflight_bootstrap_image(
    process_name: &[u8],
    image: &[u8],
) -> Result<crate::kernel::module_loader::ModuleImageSnapshot, LaunchError> {
    match crate::kernel::module_loader::snapshot_module_image(image) {
        Ok(snapshot) => Ok(snapshot),
        Err(err) => {
            crate::kernel::debug_trace::record_optional(
                "launch.bootstrap",
                "preflight_failed",
                Some(err as u64),
                false,
            );
            crate::klog_warn!(
                "[LAUNCH] snapshot rejected bootstrap image name='{}' bytes={} error={:?}",
                alloc::string::String::from_utf8_lossy(process_name),
                image.len(),
                err,
            );
            VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            SPAWN_FAILURES.fetch_add(1, Ordering::Relaxed);
            Err(LaunchError::LoaderFailed)
        }
    }
}
#[cfg(feature = "process_abstraction")]
#[cfg(not(feature = "paging_enable"))]
#[inline(always)]
fn preflight_bootstrap_snapshot(
    process_name: &[u8],
    image: &[u8],
) -> Result<crate::kernel::module_loader::ModuleImageSnapshot, LaunchError> {
    preflight_bootstrap_image(process_name, image)
}

#[cfg(feature = "process_abstraction")]
pub fn process_count() -> usize {
    PROCESS_REGISTRY.lock().len()
}

#[cfg(feature = "process_abstraction")]
pub fn process_ids_snapshot(out: &mut [ProcessId]) -> usize {
    let registry = PROCESS_REGISTRY.lock();
    let mut written = 0usize;
    for entry in registry.iter() {
        if written >= out.len() {
            break;
        }
        out[written] = entry.process_id;
        written += 1;
    }
    written
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
pub fn launch_registry_snapshot(out: &mut [LaunchRegistrySnapshotEntry]) -> usize {
    let registry = PROCESS_REGISTRY.lock();
    let mut written = 0usize;
    for entry in registry.iter() {
        if written >= out.len() {
            break;
        }
        let (_, image_pages, _, _) = entry.process.image_state();
        let (_, mapped_pages) = entry.process.mapping_state();
        out[written] = LaunchRegistrySnapshotEntry {
            process_id: entry.process_id,
            task_id: entry.task_id,
            stage: entry.stage.as_usize(),
            image_pages,
            mapped_pages,
        };
        written += 1;
    }
    written
}

#[cfg(feature = "process_abstraction")]
pub fn process_image_state(process_id: ProcessId) -> Option<(usize, usize, usize)> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| {
            let (entry_point, image_pages, image_segments, _) = entry.process.image_state();
            (entry_point, image_pages, image_segments)
        })
}

#[cfg(feature = "process_abstraction")]
pub fn process_arc_by_id(process_id: ProcessId) -> Option<alloc::sync::Arc<Process>> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.process.clone())
}

#[cfg(feature = "process_abstraction")]
pub fn process_register_mapping_typed(
    process_id: ProcessId,
    map_id: u32,
    start: u64,
    end: u64,
    prot: u32,
    flags: u32,
) -> Result<(), &'static str> {
    let registry = PROCESS_REGISTRY.lock();
    let entry = registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .ok_or(PROCESS_LOOKUP_NOT_FOUND)?;
    entry
        .process
        .register_mapping(map_id, start, end, prot, flags)
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub fn process_materialize_mapping_typed<FA>(
    process_id: ProcessId,
    start: u64,
    end: u64,
    prot: u32,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut FA,
) -> Result<(), &'static str>
where
    FA: x86_64::structures::paging::FrameAllocator<x86_64::structures::paging::Size4KiB>,
{
    let registry = PROCESS_REGISTRY.lock();
    let entry = registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .ok_or(PROCESS_LOOKUP_NOT_FOUND)?;

    // Delegate to module_loader's helper
    crate::kernel::module_loader::materialize_virtual_mapping_range(
        start,
        end,
        prot,
        page_manager,
        frame_allocator,
    )
    .map_err(|_| PROCESS_MATERIALIZE_FAILED)?;

    Ok(())
}

#[cfg(feature = "process_abstraction")]
pub fn process_mapping_state(process_id: ProcessId) -> Option<(usize, usize)> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.process.mapping_state())
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn process_launch_context_typed(process_id: ProcessId) -> Option<LaunchContext> {
    let registry = PROCESS_REGISTRY.lock();
    let entry = registry
        .iter()
        .find(|entry| entry.process_id == process_id)?;
    Some(build_context(
        entry.process_id,
        &entry.process,
        entry.task_id,
    ))
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn process_boot_image_typed(process_id: ProcessId) -> Option<Vec<u8>> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.boot_image.to_vec())
}

#[cfg(feature = "process_abstraction")]
pub fn claim_next_launch_context() -> Option<LaunchContext> {
    CLAIM_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    for entry in registry.iter_mut() {
        if entry.stage == LaunchStage::Pending {
            entry.stage = LaunchStage::Claimed;
            entry.stage_epoch = now_epoch;
            CLAIM_SUCCESS.fetch_add(1, Ordering::Relaxed);
            return Some(build_context(
                entry.process_id,
                &entry.process,
                entry.task_id,
            ));
        }
    }

    CLAIM_FAILURES.fetch_add(1, Ordering::Relaxed);
    None
}

#[cfg(feature = "process_abstraction")]
pub fn acknowledge_launch_context_typed(process_id: ProcessId, success: bool) -> bool {
    HANDOFF_ACK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    let Some(entry) = registry
        .iter_mut()
        .find(|entry| entry.process_id == process_id)
    else {
        HANDOFF_ACK_FAILURES.fetch_add(1, Ordering::Relaxed);
        return false;
    };

    entry.stage = if success {
        LaunchStage::Ready
    } else {
        LaunchStage::Pending
    };
    entry.stage_epoch = now_epoch;
    HANDOFF_ACK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    true
}

#[cfg(feature = "process_abstraction")]
pub fn launch_context_stage_typed(process_id: ProcessId) -> Option<usize> {
    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.stage.as_usize())
}

#[cfg(feature = "process_abstraction")]
pub fn consume_ready_launch_context() -> Option<LaunchContext> {
    HANDOFF_CONSUME_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let mut registry = PROCESS_REGISTRY.lock();
    recycle_stale_handoffs(&mut registry, now_epoch);
    let Some(index) = registry
        .iter()
        .position(|entry| entry.stage == LaunchStage::Ready)
    else {
        HANDOFF_CONSUME_FAILURES.fetch_add(1, Ordering::Relaxed);
        return None;
    };

    let entry = registry.remove(index);
    HANDOFF_CONSUME_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Some(build_context(
        entry.process_id,
        &entry.process,
        entry.task_id,
    ))
}

#[cfg(feature = "process_abstraction")]
pub fn execute_ready_launch_context_on_current_cpu() -> Option<LaunchContext> {
    HANDOFF_EXECUTE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let now_epoch = next_handoff_epoch();
    let candidate = {
        let mut registry = PROCESS_REGISTRY.lock();
        recycle_stale_handoffs(&mut registry, now_epoch);
        let Some(entry) = registry
            .iter_mut()
            .find(|entry| entry.stage == LaunchStage::Ready)
        else {
            HANDOFF_EXECUTE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        entry.stage_epoch = now_epoch;
        build_context(entry.process_id, &entry.process, entry.task_id)
    };

    let Some(cpu) = (unsafe { CpuLocal::try_get() }) else {
        HANDOFF_EXECUTE_FAILURES.fetch_add(1, Ordering::Relaxed);
        return None;
    };

    let task_found = {
        let mut scheduler = cpu.scheduler.lock();
        match scheduler.get_task_mut(candidate.task_id) {
            Some(task_arc) => {
                let mut task = task_arc.lock();
                task.state = TaskState::Running;
                if let Some(process) = process_arc_by_id(candidate.process_id) {
                    process.mark_running();
                }

                #[cfg(feature = "ring_protection")]
                cpu.kernel_stack_top
                    .store(task.kernel_stack_pointer as usize, Ordering::Relaxed);

                true
            }
            None => false,
        }
    };

    if !task_found {
        HANDOFF_EXECUTE_FAILURES.fetch_add(1, Ordering::Relaxed);
        return None;
    }

    cpu.current_task
        .store(candidate.task_id.0, Ordering::Relaxed);
    crate::kernel::rt_preemption::request_forced_reschedule();

    {
        let mut registry = PROCESS_REGISTRY.lock();
        if let Some(index) = registry.iter().position(|entry| {
            entry.process_id == candidate.process_id
                && entry.task_id == candidate.task_id
                && entry.stage == LaunchStage::Ready
        }) {
            registry.remove(index);
        }
    }

    HANDOFF_EXECUTE_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Some(candidate)
}

#[cfg(feature = "process_abstraction")]
pub fn process_id_by_task(task_id: TaskId) -> Option<ProcessId> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.task_id == task_id)
        .map(|entry| entry.process_id)
}

#[cfg(feature = "process_abstraction")]
pub fn current_process_arc() -> Option<Arc<Process>> {
    let tid =
        unsafe { CpuLocal::try_get().map(|cpu| TaskId(cpu.current_task.load(Ordering::Relaxed))) }?;

    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|e| e.task_id == tid)
        .map(|e| e.process.clone())
}

#[cfg(all(feature = "process_abstraction", feature = "posix_mman"))]
pub fn refresh_all_linux_runtime_vvar() {
    let processes: Vec<Arc<Process>> = {
        let registry = PROCESS_REGISTRY.lock();
        registry.iter().map(|entry| entry.process.clone()).collect()
    };
    for process in processes {
        let _ = process.refresh_linux_runtime_vvar();
    }
}

#[cfg(feature = "process_abstraction")]
pub fn terminate_process_with_status(process_id: ProcessId, status: i32) -> bool {
    TERMINATE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let (task_id, process_arc) = {
        let mut registry = PROCESS_REGISTRY.lock();
        let Some(index) = registry
            .iter()
            .position(|entry| entry.process_id == process_id)
        else {
            TERMINATE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return false;
        };
        let entry = registry.remove(index);
        (entry.task_id, entry.process)
    };

    let shared_object_fini =
        crate::kernel::dynamic_linker::api::drain_pending_shared_object_fini_reports_for_process(
            process_id,
        );
    if !shared_object_fini.is_empty() {
        let pending_calls = shared_object_fini
            .iter()
            .map(|report| report.fini_calls.len())
            .sum::<usize>();
        for report in &shared_object_fini {
            process_arc.append_deferred_fini_calls(&report.fini_calls);
        }
        klog_info!(
            "process exit: pid={} deferred_shared_object_fini_reports={} total_calls={}",
            process_id.0,
            shared_object_fini.len(),
            pending_calls,
        );
    }

    let runtime_contract = process_arc.runtime_contract_snapshot();
    if !runtime_contract.fini_calls.is_empty() {
        let mut fini_preview = alloc::string::String::new();
        for (idx, addr) in runtime_contract.fini_calls.iter().take(4).enumerate() {
            if idx != 0 {
                fini_preview.push(',');
            }
            let _ = core::fmt::Write::write_fmt(&mut fini_preview, format_args!("{:#x}", addr));
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
        RUNTIME_FINI_TRAMPOLINES_SEEN.fetch_add(1, Ordering::Relaxed);
        RUNTIME_FINI_EXECUTION_DEFERRED.fetch_add(1, Ordering::Relaxed);
        klog_info!(
            "process exit: pid={} exec='{}' runtime_fini_entry={:#x} pending_fini_hooks={} execution=deferred",
            process_id.0,
            runtime_contract.exec_path.as_str(),
            runtime_contract.runtime_fini_entry,
            runtime_contract.fini_calls.len(),
        );
    }

    process_arc.mark_exited(status);
    process_arc.clear_runtime_contract();
    finalize_task_user_exit_state(task_id);

    let cpus = crate::hal::smp::CPUS.lock();
    for cpu in cpus.iter() {
        let mut scheduler = cpu.scheduler.lock();
        scheduler.remove_task(task_id);
    }

    TERMINATE_SUCCESS.fetch_add(1, Ordering::Relaxed);
    true
}

#[cfg(feature = "process_abstraction")]
pub fn terminate_task(task_id: TaskId) -> bool {
    TERMINATE_BY_TASK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let process_id = {
        let registry = PROCESS_REGISTRY.lock();
        registry
            .iter()
            .find(|entry| entry.task_id == task_id)
            .map(|entry| entry.process_id)
    };

    let Some(process_id) = process_id else {
        TERMINATE_BY_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
        return false;
    };

    if terminate_process_with_status(process_id, 0) {
        TERMINATE_BY_TASK_SUCCESS.fetch_add(1, Ordering::Relaxed);
        true
    } else {
        TERMINATE_BY_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
        false
    }
}

#[cfg(feature = "process_abstraction")]
fn publish_bootstrap_process_and_task(
    process: alloc::sync::Arc<Process>,
    task: alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
    task_id: TaskId,
    registry_boot_image: BootImageRecord,
) -> Result<(usize, usize), LaunchError> {
    let process_id = process.id.0;
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "publish_begin",
        Some(process_id as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap register process begin\n",
    );
    let proc_ref = register_process(process);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap register process returned\n",
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap registry image begin\n",
    );
    register_process_with_task_image(proc_ref, task_id, registry_boot_image);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap registry image returned\n",
    );
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "registry_image_returned",
        Some(task_id.0 as u64),
        false,
    );

    let cpu = match unsafe { CpuLocal::try_get() } {
        Some(cpu) => cpu,
        None => {
            ENQUEUE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(LaunchError::SchedulerUnavailable);
        }
    };

    crate::kernel::task::register_task_arc(task.clone());
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "register_task_returned",
        Some(task_id.0 as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap register task returned\n",
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap scheduler lock begin\n",
    );
    let mut scheduler = cpu.scheduler.lock();
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap scheduler lock returned\n",
    );
    scheduler.add_task(task.clone());
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "scheduler_add_returned",
        Some(task_id.0 as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap scheduler add returned\n",
    );

    crate::kernel::rt_preemption::request_forced_reschedule();
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "forced_reschedule_requested",
        Some(task_id.0 as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap forced reschedule requested\n",
    );

    LAST_TASK_ID.store(task_id.0, Ordering::Relaxed);
    SPAWN_SUCCESS.fetch_add(1, Ordering::Relaxed);
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "spawn_returned",
        Some(process_id as u64),
        false,
    );
    Ok((process_id, task_id.0))
}

#[cfg(feature = "process_abstraction")]
struct BootstrapLaunchRequest<'a> {
    process_name: &'a [u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
}

#[cfg(feature = "process_abstraction")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BootstrapLaunchDecision {
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug)]
struct PreparedAlignedStaticBootstrap {
    boot_image: BootImageRecord,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug)]
struct PreparedAlignedStaticDispatch {
    prepared_bootstrap: PreparedAlignedStaticBootstrap,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn bootstrap_launch_request<'a>(
    process_name: &'a [u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> BootstrapLaunchRequest<'a> {
    BootstrapLaunchRequest {
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn bootstrap_launch_decision(request: &BootstrapLaunchRequest<'_>) -> BootstrapLaunchDecision {
    BootstrapLaunchDecision {
        priority: request.priority,
        deadline: request.deadline,
        burst_time: request.burst_time,
        kernel_stack_top: request.kernel_stack_top,
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_bootstrap_launch_entry<'a>(
    process_name: &'a [u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> (BootstrapLaunchRequest<'a>, BootstrapLaunchDecision) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap wrapper entered\n");
    let request = bootstrap_launch_request(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap request ready\n");
    let decision = bootstrap_launch_decision(&request);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap decision ready\n");
    (request, decision)
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_aligned_static_bootstrap(image: &'static [u8]) -> PreparedAlignedStaticBootstrap {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch aligned static image wrapper begin\n",
    );
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "aligned_static_borrow_begin", None, false);
    let boot_image = aligned_static_boot_image_record(image);
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "aligned_static_borrow_returned", None, false);
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "aligned_static_record_ready", None, false);
    PreparedAlignedStaticBootstrap { boot_image }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_aligned_static_dispatch(image: &'static [u8]) -> PreparedAlignedStaticDispatch {
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "aligned_static_prepare_begin", None, false);
    let prepared_bootstrap = prepare_aligned_static_bootstrap(image);
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "aligned_static_prepare_returned", None, false);
    PreparedAlignedStaticDispatch { prepared_bootstrap }
}

#[cfg(feature = "process_abstraction")]
fn spawn_bootstrap_from_image_record(
    process_name: &[u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    let (request, decision) = prepare_bootstrap_launch_entry(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    );
    let BootstrapLaunchRequest {
        process_name,
        boot_image,
        priority: _,
        deadline: _,
        burst_time: _,
        kernel_stack_top: _,
    } = request;
    let BootstrapLaunchDecision {
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    } = decision;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap entered\n");
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "entered", None, false);
    SPAWN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "paging_enable")]
    let image = prepare_bootstrap_launch_preflight(process_name, &boot_image)?;
    #[cfg(not(feature = "paging_enable"))]
    let (image, precomputed_snapshot) =
        prepare_bootstrap_launch_preflight(process_name, &boot_image)?;
    record_launch_image_preview(image);

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap task id alloc begin\n",
    );
    let task_id = allocate_task_id();
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "task_id_allocated",
        Some(task_id.0 as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap task id allocated\n",
    );
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    let kernel_stack_top = if kernel_stack_top == 0 {
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] launch bootstrap kernel stack alloc begin\n",
        );
        let stack_top = crate::hal::x86_64::smp::allocate_kernel_stack_top() as u64;
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] launch bootstrap kernel stack alloc returned\n",
        );
        stack_top
    } else {
        kernel_stack_top
    };
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap task build begin\n",
    );
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    let kernel_stack_top = kernel_stack_top;
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    let _ = kernel_stack_top;
    #[cfg(all(feature = "ring_protection", target_arch = "aarch64"))]
    let kernel_stack_top = if kernel_stack_top == 0 {
        crate::hal::aarch64::smp::allocate_kernel_stack_top() as u64
    } else {
        kernel_stack_top
    };
    #[cfg(not(feature = "ring_protection"))]
    let kernel_stack_top = kernel_stack_top;

    #[cfg(target_os = "none")]
    let irq_flags = crate::hal::HAL::irq_save();
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap irq hold begin\n",
    );

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap task create call begin\n",
    );

    #[cfg(feature = "paging_enable")]
    let create_result = if let Some(hhdm) = crate::hal::hhdm_offset() {
        unsafe {
            let offset = x86_64::VirtAddr::new(hhdm);
            let lvl4 = crate::kernel::memory::paging::active_level_4_table(offset);
            let mut page_manager =
                crate::kernel::memory::paging::PageManager::new(offset, lvl4);
            let mut frame_allocator = crate::kernel::vmm::PageAllocWrapper;
            Process::materialize_bootstrap_task_from_image(
                process_name,
                image,
                task_id,
                priority,
                deadline,
                burst_time,
                kernel_stack_top,
                current_cr3_phys(),
                &mut page_manager,
                &mut frame_allocator,
            )
        }
    } else {
        Process::create_bootstrap_task_from_image(
            process_name,
            image,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
            current_cr3_phys(),
        )
    };

    #[cfg(not(feature = "paging_enable"))]
    let create_result = Process::create_bootstrap_task_from_snapshot(
        process_name,
        image,
        precomputed_snapshot,
        task_id,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    );

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap task create call returned\n",
    );

    let (process, task) = match create_result {
        Ok(v) => v,
        Err(err) => {
            crate::kernel::debug_trace::record_optional(
                "launch.bootstrap",
                "task_build_failed",
                Some(process_prepare_error_code(err)),
                false,
            );
            crate::klog_warn!(
                "[LAUNCH] bootstrap task creation failed name='{}' bytes={} error={:?}",
                alloc::string::String::from_utf8_lossy(process_name),
                image.len(),
                err,
            );
            SPAWN_FAILURES.fetch_add(1, Ordering::Relaxed);
            #[cfg(target_os = "none")]
            crate::hal::HAL::irq_restore(irq_flags);
            return Err(LaunchError::LoaderFailed);
        }
    };
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "task_build_returned", None, false);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap task build returned\n",
    );

    #[cfg(target_os = "none")]
    crate::hal::HAL::irq_restore(irq_flags);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap irq hold returned\n",
    );

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap registry image clone begin\n");
    let registry_boot_image = boot_image.clone();
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap registry image clone returned\n");

    let publish_result =
        publish_bootstrap_process_and_task(process, task, task_id, registry_boot_image);

    publish_result
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_bootstrap_launch_image<'a>(
    process_name: &[u8],
    boot_image: &'a BootImageRecord,
) -> Result<&'a [u8], LaunchError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap image prep begin\n");
    let (max_name_len, max_boot_image_bytes) = bootstrap_launch_limits();
    let image = bootstrap_image_slice(boot_image);

    if validate_bootstrap_request(process_name, image, max_name_len, max_boot_image_bytes).is_err()
    {
        crate::kernel::debug_trace::record_optional(
            "launch.bootstrap",
            "validation_failed",
            Some(image.len() as u64),
            false,
        );
        VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
        SPAWN_FAILURES.fetch_add(1, Ordering::Relaxed);
        return Err(LaunchError::InvalidSpawnRequest);
    }
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap validation returned\n");
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "validation_returned",
        Some(image.len() as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap image prep returned\n");
    Ok(image)
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn begin_bootstrap_preflight_window() {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch preflight win\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap preflight window begin\n");
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn finish_bootstrap_preflight_window() {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch preflight ok\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap preflight window returned\n");
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
#[inline(always)]
fn prepare_bootstrap_launch_preflight<'a>(
    process_name: &[u8],
    boot_image: &'a BootImageRecord,
) -> Result<&'a [u8], LaunchError> {
    begin_bootstrap_preflight_window();
    let image = prepare_bootstrap_launch_image(process_name, boot_image)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch preflight run\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap preflight begin\n");
    preflight_bootstrap_image(process_name, image)?;
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "preflight_returned", None, false);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap preflight returned\n",
    );
    finish_bootstrap_preflight_window();
    Ok(image)
}

#[cfg(all(feature = "process_abstraction", not(feature = "paging_enable")))]
#[inline(always)]
fn prepare_bootstrap_launch_preflight<'a>(
    process_name: &[u8],
    boot_image: &'a BootImageRecord,
) -> Result<(&'a [u8], crate::kernel::module_loader::ModuleImageSnapshot), LaunchError> {
    begin_bootstrap_preflight_window();
    let image = prepare_bootstrap_launch_image(process_name, boot_image)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch preflight run\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap preflight begin\n");
    let precomputed_snapshot = preflight_bootstrap_snapshot(process_name, image)?;
    crate::kernel::debug_trace::record_optional("launch.bootstrap", "preflight_returned", None, false);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap preflight returned\n",
    );
    finish_bootstrap_preflight_window();
    Ok((image, precomputed_snapshot))
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn bootstrap_launch_limits() -> (usize, usize) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap config begin\n");
    let limits = (
        crate::config::KernelConfig::launch_max_process_name_len(),
        crate::config::KernelConfig::launch_max_boot_image_bytes(),
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap config returned\n");
    limits
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn bootstrap_image_slice(boot_image: &BootImageRecord) -> &[u8] {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap image slice begin\n");
    let image = boot_image.as_slice();
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap image slice returned\n");
    image
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn validate_bootstrap_request(
    process_name: &[u8],
    image: &[u8],
    max_name_len: usize,
    max_boot_image_bytes: usize,
) -> Result<(), LaunchError> {
    if process_name.is_empty()
        || process_name.len() > max_name_len
        || image.is_empty()
        || image.len() > max_boot_image_bytes
    {
        Err(LaunchError::InvalidSpawnRequest)
    } else {
        Ok(())
    }
}

#[cfg(feature = "process_abstraction")]
pub fn spawn_bootstrap_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch owned image wrapper begin\n");
    spawn_bootstrap_from_image_record(
        process_name,
        BootImageRecord::Owned(image.to_vec()),
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

#[cfg(feature = "process_abstraction")]
pub fn spawn_bootstrap_from_static_image(
    process_name: &[u8],
    image: &'static [u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch static image wrapper begin\n");
    spawn_bootstrap_from_image_record(
        process_name,
        BootImageRecord::OwnedAligned(
            {
                let words = image.len().div_ceil(core::mem::size_of::<u64>());
                let mut storage = alloc::vec![0u64; words];
                let byte_len = storage.len() * core::mem::size_of::<u64>();
                let bytes = unsafe {
                    core::slice::from_raw_parts_mut(storage.as_mut_ptr() as *mut u8, byte_len)
                };
                bytes[..image.len()].copy_from_slice(image);
                storage
            },
            image.len(),
        ),
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

#[cfg(all(test, feature = "process_abstraction"))]
#[path = "process_runtime_validation_tests.rs"]
mod validation_tests;


