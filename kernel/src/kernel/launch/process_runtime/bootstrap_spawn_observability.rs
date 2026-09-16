use super::*;
use crate::observability_launch;

#[cfg(feature = "process_abstraction")]
pub fn log_spawn_begin(
    _process_name: &[u8],
    _image_len: usize,
    _priority: u8,
    _deadline: u64,
    _burst_time: u64,
    _kernel_stack_top: u64,
    _interpreter_len: usize,
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
    _process_name: &[u8],
    _result: &Result<(usize, usize), LaunchError>,
    _image_len: usize,
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
