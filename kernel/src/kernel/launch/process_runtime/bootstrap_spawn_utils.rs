use super::*;
use core::sync::atomic::Ordering;
use crate::observability_launch;

#[cfg(feature = "process_abstraction")]
pub fn validate_spawn_request(process_name: &[u8], boot_image: &BootImageRecord) -> Result<(), LaunchError> {
    if process_name.is_empty() || boot_image.as_slice().is_empty() {
        SPAWN_FAILURES.fetch_add(1, Ordering::Relaxed);
        observability_launch! {
            crate::klog_warn!(
                "bootstrap spawn rejected: empty name_or_image name_len={} image_bytes={}",
                process_name.len(),
                boot_image.as_slice().len(),
            );
        }
        return Err(LaunchError::InvalidSpawnRequest);
    }
    Ok(())
}

#[cfg(feature = "process_abstraction")]
pub fn log_spawn_record(_name_str: &str, _boot_len: usize, _priority: u8, _deadline: u64, _burst_time: u64, _kernel_stack_top: u64) {
    observability_launch! {
        crate::klog_info!(
            "bootstrap spawn record: name='{}' image_bytes={} priority={} deadline={} burst={} kstack={:#x}",
            name_str,
            boot_len,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        );
    }
}
