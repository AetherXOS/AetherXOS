use super::*;

pub mod support;
pub mod wrappers;
pub mod query;
pub mod lifecycle;
pub mod bootstrap;
pub mod bootstrap_spawn;

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
pub use support::{
    process_prepare_error_code, process_register_mapping_typed, process_materialize_mapping_typed,
    process_launch_context_typed, process_boot_image_typed, refresh_all_linux_runtime_vvar,
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

// helper functions moved to `support.rs`
