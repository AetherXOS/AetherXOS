use super::*;

#[inline(always)]
pub(super) fn record_launch_image_preview(image: &[u8]) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap image preview begin\n");
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        crate::kernel::debug_trace::record_bytes_preview("launch.image", "preview", image);
    }
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap image preview returned\n");
}

#[inline(always)]
pub(super) fn aligned_static_boot_image_record(image: &'static [u8]) -> BootImageRecord {
    BootImageRecord::BorrowedStatic(image)
}

#[inline(always)]
fn dispatch_bootstrap_image_record(
    process_name: &[u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    invoke_bootstrap_dispatch_call(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

#[inline(always)]
pub(super) fn invoke_bootstrap_dispatch_call(
    process_name: &[u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch bootstrap dispatch call begin\n");
    let result = spawn_bootstrap_from_image_record(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch bootstrap dispatch call returned\n",
    );
    result
}

#[inline(always)]
pub(super) fn invoke_bootstrap_image_record_dispatch(
    process_name: &[u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    dispatch_bootstrap_image_record(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

#[inline(always)]
pub(super) fn dispatch_aligned_static_bootstrap(
    process_name: &[u8],
    image: &'static [u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    let prepared = prepare_aligned_static_dispatch(image);
    invoke_aligned_static_dispatch(
        process_name,
        prepared.prepared_bootstrap.boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

#[inline(always)]
pub(super) fn invoke_aligned_static_dispatch(
    process_name: &[u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch aligned static image dispatch begin\n",
    );
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "aligned_static_dispatch_call",
        None,
        false,
    );
    let result = invoke_bootstrap_image_record_dispatch(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch aligned static image dispatch returned\n",
    );
    result
}

#[inline(always)]
pub(super) fn invoke_aligned_static_bootstrap(
    process_name: &[u8],
    image: &'static [u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch aligned static image invoke begin\n");
    crate::kernel::debug_trace::record_optional(
        "launch.bootstrap",
        "aligned_static_invoke_call",
        None,
        false,
    );
    let result = dispatch_aligned_static_bootstrap(
        process_name,
        image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] launch aligned static image invoke returned\n",
    );
    result
}

pub fn spawn_bootstrap_from_aligned_static_image(
    process_name: &[u8],
    image: &'static [u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    invoke_aligned_static_bootstrap(
        process_name,
        image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

pub fn clone_process_from_registered_image(
    source_process_id: ProcessId,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<(usize, usize), LaunchError> {
    let (name, image) = {
        let registry = PROCESS_REGISTRY.lock();
        let Some(entry) = registry
            .iter()
            .find(|entry| entry.process_id == source_process_id)
        else {
            return Err(LaunchError::InvalidSpawnRequest);
        };

        let name_lock = entry.process.name.lock();
        let mut name_end = name_lock.len();
        while name_end > 0 && name_lock[name_end - 1] == 0 {
            name_end -= 1;
        }
        let name_bytes = if name_end == 0 {
            b"forked".to_vec()
        } else {
            name_lock[..name_end].to_vec()
        };

        let image = entry.boot_image.to_vec();
        if image.is_empty() {
            return Err(LaunchError::LoaderFailed);
        }
        (name_bytes, image)
    };

    spawn_bootstrap_from_image(
        &name,
        &image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}
