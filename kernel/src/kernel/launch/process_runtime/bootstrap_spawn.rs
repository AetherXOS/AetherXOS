use super::*;
use alloc::sync::Arc;
use core::sync::atomic::Ordering;
use crate::interfaces::task::TaskId;
use crate::kernel::cpu_local::CpuLocal;
use crate::kernel::process::Process;
use crate::kernel::launch::process_runtime::bootstrap_dispatch::record_launch_image_preview;
use crate::observability_launch;

#[cfg(feature = "process_abstraction")]
pub fn spawn_bootstrap_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    interpreter_image: Option<alloc::vec::Vec<u8>>,
) -> Result<(usize, usize), LaunchError> {

    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.bootstrap",
            "spawn_begin",
            Some(image.len() as u64),
            false,
        );
        crate::klog_info!(
            "bootstrap spawn begin: name='{}' image_bytes={} priority={} deadline={} burst={} kstack={:#x} interp={}",
            alloc::string::String::from_utf8_lossy(process_name),
            image.len(),
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
            interpreter_image.as_ref().map(|img| img.len()).unwrap_or(0),
        );
    }
    record_launch_image_preview(image);
    let boot_image = BootImageRecord::Owned(image.to_vec());
    let result = spawn_bootstrap_from_image_record(
        process_name,
        boot_image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
        interpreter_image,
    );
    observability_launch! {
        if let Ok((process_id, task_id)) = result {
            crate::kernel::debug_trace::record_optional(
                "launch.bootstrap",
                "spawn_ok",
                Some(process_id as u64),
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
                Some(image.len() as u64),
                false,
            );
            crate::klog_warn!(
                "bootstrap spawn failed: name='{}' image_bytes={}'",
                alloc::string::String::from_utf8_lossy(process_name),
                image.len(),
            );
        }
    }
    result
}

#[cfg(feature = "process_abstraction")]
pub fn spawn_bootstrap_from_image_record(
    process_name: &[u8],
    boot_image: BootImageRecord,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
    _interpreter_image: Option<alloc::vec::Vec<u8>>,
) -> Result<(usize, usize), LaunchError> {
    SPAWN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    observability_launch! {
        crate::kernel::debug_trace::record_optional(
            "launch.bootstrap",
            "spawn_record_begin",
            Some(boot_image.as_slice().len() as u64),
            false,
        );
    }

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

    let name_str = alloc::string::String::from_utf8_lossy(process_name);
    observability_launch! {
        crate::klog_info!(
            "bootstrap spawn record: name='{}' image_bytes={} priority={} deadline={} burst={} kstack={:#x}",
            name_str,
            boot_image.as_slice().len(),
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        );
    }

    #[cfg(feature = "paging_enable")]
    let cr3 = {
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        let offset = x86_64::VirtAddr::new(hhdm);
        let _active_lvl4 = crate::kernel::memory::paging::active_level_4_table(offset.as_u64());
        #[cfg(target_os = "none")]
        {
            x86_64::registers::control::Cr3::read().0.start_address()
        }
        #[cfg(not(target_os = "none"))]
        {
            x86_64::PhysAddr::new(0)
        }
    };

    #[cfg(not(feature = "paging_enable"))]
    let _cr3 = x86_64::PhysAddr::new(0);

    #[cfg(feature = "paging_enable")]
    let process = Arc::new(Process::new_with_cr3(name_str.as_bytes(), cr3));
    #[cfg(not(feature = "paging_enable"))]
    let process = Arc::new(Process::new(name_str.as_bytes()));
    
    let process_id = process.id;
    let task_id = TaskId(process_id.0);

    let image_bytes = boot_image.as_slice();

    #[cfg(feature = "paging_enable")]
    let task = {
        let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
        let offset = x86_64::VirtAddr::new(hhdm);
        let lvl4 = unsafe { &mut *( (cr3.as_u64() + hhdm) as *mut x86_64::structures::paging::PageTable ) };
        let mut page_manager = crate::kernel::memory::paging::PageManager {
            mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
            physical_memory_offset: offset,
        };
        let mut frame_allocator = crate::hal::HAL::create_frame_allocator();

        let prepared = crate::kernel::module_loader::materialize_and_build_process_bootstrap_task(
            &process,
            image_bytes,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
            &mut page_manager,
            &mut frame_allocator,
        ).map_err(|_| {
            observability_launch! {
                crate::kernel::debug_trace::record_optional(
                    "launch.bootstrap",
                    "materialize_failed",
                    Some(process_id.0 as u64),
                    false,
                );
                crate::klog_warn!(
                    "bootstrap materialize failed: pid={} name='{}' image_bytes={}",
                    process_id.0,
                    name_str,
                    image_bytes.len(),
                );
            }
            LaunchError::LoaderFailed
        })?;

        if let Some(interp) = _interpreter_image {
            let interp_prepared = crate::kernel::module_loader::materialize_process_image(
                &process,
                &interp,
                &mut page_manager,
                &mut frame_allocator,
            ).map_err(|_| LaunchError::LoaderFailed)?;

            process.set_interpreter_base(interp_prepared.load_plan.aslr_base);
            process.set_runtime_entry(Some(interp_prepared.load_plan.entry + interp_prepared.load_plan.aslr_base));
        }
        
        prepared
    };


    #[cfg(not(feature = "paging_enable"))]
    let task = {
        crate::kernel::module_loader::build_process_bootstrap_task(
            &process,
            image_bytes,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        ).map_err(|_| LaunchError::LoaderFailed)?
    };

    publish_bootstrap_process_and_task(process, task, task_id, boot_image)
}
