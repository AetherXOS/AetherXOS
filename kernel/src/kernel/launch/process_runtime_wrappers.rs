use super::*;

#[cfg(feature = "process_abstraction")]
pub fn process_launch_context(process_id: usize) -> Option<LaunchContext> {
    process_launch_context_typed(ProcessId(process_id))
}

#[cfg(feature = "process_abstraction")]
pub fn process_boot_image(process_id: usize) -> Option<Vec<u8>> {
    process_boot_image_typed(ProcessId(process_id))
}

#[cfg(feature = "process_abstraction")]
pub fn acknowledge_launch_context(process_id: usize, success: bool) -> bool {
    acknowledge_launch_context_typed(ProcessId(process_id), success)
}

#[cfg(feature = "process_abstraction")]
pub fn launch_context_stage(process_id: usize) -> Option<usize> {
    launch_context_stage_typed(ProcessId(process_id))
}

#[cfg(feature = "process_abstraction")]
pub fn terminate_process(process_id: ProcessId) -> bool {
    terminate_process_with_status(process_id, 0)
}
