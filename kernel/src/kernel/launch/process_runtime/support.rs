use super::*;

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
pub fn process_prepare_error_code(
    err: crate::kernel::module_loader::ProcessPrepareError,
) -> u64 {
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
pub fn process_materialize_mapping_typed(
    process_id: ProcessId,
    start: u64,
    end: u64,
    prot: u32,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut crate::hal::paging::PageAllocWrapper,
) -> Result<(), &'static str> {
    let registry = PROCESS_REGISTRY.lock();
    let _entry = registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .ok_or(PROCESS_LOOKUP_NOT_FOUND)?;

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
pub fn process_launch_context_typed(process_id: ProcessId) -> Option<LaunchContext> {
    let registry = PROCESS_REGISTRY.lock();
    let entry = registry
        .iter()
        .find(|entry| entry.process_id == process_id)?;
    Some(build_context(entry.process_id, &entry.process, entry.task_id))
}

#[cfg(feature = "process_abstraction")]
pub fn process_boot_image_typed(process_id: ProcessId) -> Option<Vec<u8>> {
    let registry = PROCESS_REGISTRY.lock();
    registry
        .iter()
        .find(|entry| entry.process_id == process_id)
        .map(|entry| entry.boot_image.to_vec())
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
pub(super) fn recycle_stale_handoffs(registry: &mut [LaunchRegistryEntry], now_epoch: u64) {
    STALE_SCAN_CALLS.fetch_add(1, Ordering::Relaxed);
    let timeout_epochs = crate::config::KernelConfig::launch_handoff_stage_timeout_epochs();

    let mut recycled = 0u64;
    for entry in registry.iter_mut() {
        let age = now_epoch.saturating_sub(entry.stage_epoch);
        if entry.stage == LaunchStage::Claimed && age >= timeout_epochs {
            entry.stage = LaunchStage::Pending;
            entry.stage_epoch = now_epoch;
            STALE_CLAIM_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            recycled = recycled.saturating_add(1);
        } else if entry.stage == LaunchStage::Ready && age >= timeout_epochs {
            entry.stage = LaunchStage::Pending;
            entry.stage_epoch = now_epoch;
            STALE_READY_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
            recycled = recycled.saturating_add(1);
        }
    }

    if recycled != 0 {
        STALE_RECYCLED_ENTRIES.fetch_add(recycled, Ordering::Relaxed);
    }
}

#[cfg(feature = "paging_enable")]
#[inline(always)]
pub(super) fn current_cr3_phys() -> x86_64::PhysAddr {
    use crate::interfaces::cpu::CpuRegisters;
    let root = crate::hal::cpu::ArchCpuRegisters::read_page_table_root();
    x86_64::PhysAddr::new(root)
}

#[cfg(feature = "process_abstraction")]
pub(super) fn register_process(process: Arc<Process>) -> Arc<Process> {
    process
}

#[cfg(feature = "process_abstraction")]
pub(super) fn register_process_with_task_image(
    process: Arc<Process>,
    task_id: TaskId,
    boot_image: BootImageRecord,
) {
    crate::kernel::debug_trace::record_with_metadata(
        "launch.registry",
        "begin",
        Some(task_id.0 as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Launch,
    );
    process.as_ref().mark_runnable();
    crate::kernel::debug_trace::record_with_metadata(
        "launch.registry",
        "mark_runnable_returned",
        Some(process.id.0 as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Launch,
    );
    let now_epoch = next_handoff_epoch();
    PROCESS_REGISTRY.lock().push(LaunchRegistryEntry {
        process_id: process.id,
        process,
        task_id,
        boot_image,
        stage: LaunchStage::Pending,
        stage_epoch: now_epoch,
    });
    crate::kernel::debug_trace::record_with_metadata(
        "launch.registry",
        "returned",
        Some(now_epoch),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Launch,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] launch registry helper returned\n");
}

#[cfg(feature = "process_abstraction")]
pub(super) fn build_context(
    process_id: ProcessId,
    process: &Arc<Process>,
    task_id: TaskId,
) -> LaunchContext {
    let (entry, image_pages, image_segments, exec_generation) = process.image_state();
    let (mapped_regions, mapped_pages) = process.mapping_state();

    #[cfg(feature = "paging_enable")]
    let cr3 = process.cr3.as_u64() as usize;
    #[cfg(not(feature = "paging_enable"))]
    let cr3 = 0;

    LaunchContext {
        process_id,
        task_id,
        entry,
        image_pages,
        image_segments,
        exec_generation,
        mapped_regions,
        mapped_pages,
        cr3,
    }
}
