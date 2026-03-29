use core::sync::atomic::Ordering;

use super::{
    ModuleLoaderStats, ProcessPrepareError, BOOTSTRAP_TASK_ATTEMPTS, BOOTSTRAP_TASK_FAILURES,
    BOOTSTRAP_TASK_SUCCESS, LAST_PREFLIGHT_FINGERPRINT, MAP_PLAN_ATTEMPTS, MAP_PLAN_FAILURES,
    MAP_PLAN_SUCCESS, PARSE_ATTEMPTS, PARSE_FAILURES, PARSE_SUCCESS, PLAN_ATTEMPTS, PLAN_FAILURES,
    PLAN_SUCCESS, PREFLIGHT_ATTEMPTS, PREFLIGHT_FAILURES, PREFLIGHT_SUCCESS,
    SEGMENT_MATERIALIZATION_ATTEMPTS, SEGMENT_MATERIALIZATION_FAILURES,
    SEGMENT_MATERIALIZATION_SUCCESS, SEGMENT_MATERIALIZED_BYTES,
};

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn materialize_virtual_mapping_range(
    start: u64,
    end: u64,
    prot: u32,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
) -> Result<(), ProcessPrepareError> {
    use x86_64::structures::paging::PageTableFlags;

    if start >= end {
        return Err(ProcessPrepareError::MappingBindFailed);
    }

    let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if (prot & crate::modules::posix_consts::mman::PROT_WRITE) != 0 {
        flags |= PageTableFlags::WRITABLE;
    }
    if (prot & crate::modules::posix_consts::mman::PROT_EXEC) == 0 {
        flags |= x86_64::structures::paging::PageTableFlags::NO_EXECUTE;
    }

    let plan = VirtualMappingPlan {
        start,
        end,
        file_bytes: 0,
        zero_fill_bytes: 0,
    };

    page_manager
        .apply_virtual_mapping_plan(&[plan], flags, frame_allocator)
        .map_err(|_| ProcessPrepareError::PagingApplyFailed)?;

    Ok(())
}

#[cfg(feature = "process_abstraction")]
fn prepare_bootstrap_runtime_entry(
    process: &crate::kernel::process::Process,
    image: &[u8],
    task_id: crate::interfaces::TaskId,
) -> Result<u64, ProcessPrepareError> {
    crate::kernel::debug_trace::record_with_metadata(
        "loader.bootstrap_task",
        "prepare_image_begin",
        Some(task_id.0 as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader prepare process image entry begin\n",
    );

    let entry = match super::prepare_process_image_entry(process, image) {
        Ok(entry) => entry,
        Err(err) => {
            crate::kernel::debug_trace::record_fault(
                "loader.bootstrap_task",
                "prepare_image_failed",
                None,
            );
            crate::klog_warn!(
                "[LOADER] prepare_process_image_entry failed for bootstrap task: {:?}",
                err,
            );
            BOOTSTRAP_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };

    crate::kernel::debug_trace::record_with_metadata(
        "loader.bootstrap_task",
        "prepare_image_returned",
        Some(entry),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader prepare process image entry returned\n",
    );
    Ok(entry)
}

#[cfg(feature = "process_abstraction")]
fn prepare_bootstrap_runtime_entry_from_snapshot(
    process: &crate::kernel::process::Process,
    image: &[u8],
    snapshot: super::ModuleImageSnapshot,
    task_id: crate::interfaces::TaskId,
) -> Result<u64, ProcessPrepareError> {
    crate::kernel::debug_trace::record_with_metadata(
        "loader.bootstrap_task",
        "prepare_image_begin",
        Some(task_id.0 as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader prepare process image entry begin\n",
    );

    let entry = match super::prepare_process_image_entry_from_snapshot(process, image, snapshot) {
        Ok(entry) => entry,
        Err(err) => {
            crate::kernel::debug_trace::record_fault(
                "loader.bootstrap_task",
                "prepare_image_failed",
                None,
            );
            crate::klog_warn!(
                "[LOADER] prepare_process_image_entry_from_snapshot failed for bootstrap task: {:?}",
                err,
            );
            BOOTSTRAP_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };

    crate::kernel::debug_trace::record_with_metadata(
        "loader.bootstrap_task",
        "prepare_image_returned",
        Some(entry),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    Ok(entry)
}

#[cfg(feature = "process_abstraction")]
fn bootstrap_page_table_root(_process: &crate::kernel::process::Process) -> u64 {
    #[cfg(feature = "paging_enable")]
    {
        _process.cr3.as_u64()
    }
    #[cfg(not(feature = "paging_enable"))]
    {
        0
    }
}

#[cfg(feature = "process_abstraction")]
fn build_bootstrap_task_from_entry(
    task_id: crate::interfaces::TaskId,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    cr3: u64,
    entry: u64,
) -> alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>> {
    crate::kernel::debug_trace::record_with_metadata(
        "loader.bootstrap_task",
        "begin",
        Some(task_id.0 as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader.bootstrap_task begin\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader.bootstrap_task spec begin\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader.bootstrap_task spec returned\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader.bootstrap_task call begin\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] task.shared direct begin\n");
    let task = crate::interfaces::KernelTask::new_shared_bootstrap(
        task_id,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        cr3,
        entry,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] task.shared direct returned\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader.bootstrap_task call returned\n");
    crate::kernel::debug_trace::record_with_metadata(
        "loader.bootstrap_task",
        "returned",
        Some(task_id.0 as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader.bootstrap_task returned\n");
    task
}

#[cfg(feature = "process_abstraction")]
fn attach_bootstrap_thread(
    process: &crate::kernel::process::Process,
    task: &alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader bootstrap add thread begin\n",
    );
    process.add_bootstrap_thread(task.lock().id);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader bootstrap add thread returned\n",
    );
}

#[cfg(feature = "process_abstraction")]
pub fn build_process_bootstrap_task(
    process: &crate::kernel::process::Process,
    image: &[u8],
    task_id: crate::interfaces::TaskId,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<
    alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
    ProcessPrepareError,
> {
    BOOTSTRAP_TASK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let entry = prepare_bootstrap_runtime_entry(process, image, task_id)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader bootstrap cr3 begin\n");
    let cr3 = bootstrap_page_table_root(process);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader bootstrap cr3 returned\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader bootstrap build task direct begin\n",
    );
    let task = build_bootstrap_task_from_entry(
        task_id,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        cr3,
        entry,
    );
    attach_bootstrap_thread(process, &task);
    BOOTSTRAP_TASK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(task)
}

#[cfg(feature = "process_abstraction")]
pub fn build_process_bootstrap_task_from_snapshot(
    process: &crate::kernel::process::Process,
    image: &[u8],
    snapshot: super::ModuleImageSnapshot,
    task_id: crate::interfaces::TaskId,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<
    alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
    ProcessPrepareError,
> {
    BOOTSTRAP_TASK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let entry =
        prepare_bootstrap_runtime_entry_from_snapshot(process, image, snapshot, task_id)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader bootstrap cr3 begin\n");
    let cr3 = bootstrap_page_table_root(process);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] loader bootstrap cr3 returned\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader bootstrap build task direct begin\n",
    );
    let task = build_bootstrap_task_from_entry(
        task_id,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        cr3,
        entry,
    );
    attach_bootstrap_thread(process, &task);
    BOOTSTRAP_TASK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(task)
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub fn materialize_and_build_process_bootstrap_task(
    process: &crate::kernel::process::Process,
    image: &[u8],
    task_id: crate::interfaces::TaskId,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
) -> Result<
    alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
    ProcessPrepareError,
> {
    BOOTSTRAP_TASK_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    crate::kernel::debug_trace::record_kernel_context(
        "loader.bootstrap_task",
        "materialize_begin",
        Some(task_id.0 as u64),
    );

    let prepared =
        match super::materialize_process_image(process, image, page_manager, frame_allocator) {
            Ok(p) => p,
            Err(err) => {
                crate::kernel::debug_trace::record_fault(
                    "loader.bootstrap_task",
                    "materialize_failed",
                    None,
                );
                BOOTSTRAP_TASK_FAILURES.fetch_add(1, Ordering::Relaxed);
                return Err(err);
            }
    };
    crate::kernel::debug_trace::record_kernel_context(
        "loader.bootstrap_task",
        "materialize_returned",
        Some(prepared.load_plan.entry),
    );

    crate::kernel::debug_trace::record_kernel_context(
        "loader.bootstrap_task",
        "begin",
        Some(task_id.0 as u64),
    );
    let task = build_bootstrap_task_from_entry(
        task_id,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        process.cr3.as_u64(),
        prepared.load_plan.entry,
    );
    attach_bootstrap_thread(process, &task);
    BOOTSTRAP_TASK_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(task)
}

pub(super) fn stats() -> ModuleLoaderStats {
    ModuleLoaderStats {
        preflight_attempts: PREFLIGHT_ATTEMPTS.load(Ordering::Relaxed),
        preflight_success: PREFLIGHT_SUCCESS.load(Ordering::Relaxed),
        preflight_failures: PREFLIGHT_FAILURES.load(Ordering::Relaxed),
        last_preflight_fingerprint: LAST_PREFLIGHT_FINGERPRINT.load(Ordering::Relaxed),
        parse_attempts: PARSE_ATTEMPTS.load(Ordering::Relaxed),
        parse_success: PARSE_SUCCESS.load(Ordering::Relaxed),
        parse_failures: PARSE_FAILURES.load(Ordering::Relaxed),
        plan_attempts: PLAN_ATTEMPTS.load(Ordering::Relaxed),
        plan_success: PLAN_SUCCESS.load(Ordering::Relaxed),
        plan_failures: PLAN_FAILURES.load(Ordering::Relaxed),
        mapping_plan_attempts: MAP_PLAN_ATTEMPTS.load(Ordering::Relaxed),
        mapping_plan_success: MAP_PLAN_SUCCESS.load(Ordering::Relaxed),
        mapping_plan_failures: MAP_PLAN_FAILURES.load(Ordering::Relaxed),
        bootstrap_task_attempts: BOOTSTRAP_TASK_ATTEMPTS.load(Ordering::Relaxed),
        bootstrap_task_success: BOOTSTRAP_TASK_SUCCESS.load(Ordering::Relaxed),
        bootstrap_task_failures: BOOTSTRAP_TASK_FAILURES.load(Ordering::Relaxed),
        segment_materialization_attempts: SEGMENT_MATERIALIZATION_ATTEMPTS.load(Ordering::Relaxed),
        segment_materialization_success: SEGMENT_MATERIALIZATION_SUCCESS.load(Ordering::Relaxed),
        segment_materialization_failures: SEGMENT_MATERIALIZATION_FAILURES.load(Ordering::Relaxed),
        segment_materialized_bytes: SEGMENT_MATERIALIZED_BYTES.load(Ordering::Relaxed),
    }
}
