#[path = "process_lifecycle/lifecycle_ops.rs"]
mod lifecycle_ops;
#[path = "process_lifecycle/query_ops.rs"]
mod query_ops;

pub(crate) use lifecycle_ops::{
    sys_resolve_plt, sys_spawn_process, sys_terminate_process, sys_terminate_task,
};
pub(crate) use query_ops::{
    sys_get_launch_stats, sys_get_process_count, sys_get_process_id_by_task,
    sys_get_process_image_state, sys_get_process_mapping_state, sys_list_process_ids,
};
