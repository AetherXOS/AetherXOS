use super::*;
use crate::observability_launch;

#[cfg(feature = "process_abstraction")]
pub fn log_spawn_begin(
    process_name: &[u8],
    image_len: usize,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    interpreter_len: usize,
) {
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.bootstrap",
            "spawn_begin",
            Some(image_len as u64),
            false,
        );
        crate::klog_info!(
            "bootstrap spawn begin: name='{}' image_bytes={} priority={} deadline={} burst={} kstack={:#x} interp={}",
            alloc::string::String::from_utf8_lossy(process_name),
            image_len,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
            interpreter_len,
        );
    }
}

#[cfg(feature = "process_abstraction")]
pub fn log_spawn_result(
    process_name: &[u8],
    result: &Result<(usize, usize), LaunchError>,
    image_len: usize,
) {
    observability_launch! {
        if let Ok((process_id, task_id)) = result {
            crate::kernel::debug_trace::record_optional(
                "launch.bootstrap",
                "spawn_ok",
                Some(*process_id as u64),
                false,
            );
            crate::klog_info!(
                "bootstrap spawn ok: pid={} tid={} name='{}'",
                process_id,
                task_id,
                alloc::string::String::from_utf8_lossy(process_name),
            );
        } else {
            crate::kernel::debug_trace::record_optional(
                "launch.bootstrap",
                "spawn_err",
                Some(image_len as u64),
                false,
            );
            crate::klog_warn!(
                "bootstrap spawn failed: name='{}' image_bytes={}'",
                alloc::string::String::from_utf8_lossy(process_name),
                image_len,
            );
        }
    }
}
