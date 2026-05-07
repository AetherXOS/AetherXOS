use super::*;

pub mod support;
pub mod wrappers;
pub mod query;
pub mod lifecycle;
pub mod bootstrap;

#[cfg(feature = "process_abstraction")]
pub mod bootstrap_dispatch;

#[cfg(feature = "process_abstraction")]
pub use wrappers::{
    acknowledge_launch_context, launch_context_stage, process_boot_image, process_launch_context,
    terminate_process,
};
#[cfg(feature = "process_abstraction")]
pub use bootstrap_dispatch::{
    clone_process_from_registered_image, spawn_bootstrap_from_aligned_static_image,
};
#[cfg(feature = "process_abstraction")]
pub use query::{
    process_count, process_ids_snapshot, launch_registry_snapshot, process_image_state,
    process_arc_by_id, process_mapping_state, process_id_by_task, current_process_arc,
};
#[cfg(feature = "process_abstraction")]
pub use lifecycle::{
    claim_next_launch_context, acknowledge_launch_context_typed, launch_context_stage_typed,
    consume_ready_launch_context, execute_ready_launch_context_on_current_cpu,
    terminate_process_with_status, terminate_task,
};
#[cfg(feature = "process_abstraction")]
pub use bootstrap::{
    spawn_bootstrap_from_image, spawn_bootstrap_from_image_record, publish_bootstrap_process_and_task,
};

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
pub fn process_prepare_error_code(err: crate::kernel::module_loader::ProcessPrepareError) -> u64 {
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
    Some(support::build_context(
        entry.process_id,
        &entry.process,
        entry.task_id,
    ))
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
