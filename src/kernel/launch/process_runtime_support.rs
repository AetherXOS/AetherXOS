use super::*;

#[cfg(feature = "process_abstraction")]
pub(super) fn recycle_stale_handoffs(registry: &mut [RegistryEntry], now_epoch: u64) {
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
    frame.start_address()
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
    process.mark_runnable();
    crate::kernel::debug_trace::record_with_metadata(
        "launch.registry",
        "mark_runnable_returned",
        Some(process.id.0 as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Launch,
    );
    let now_epoch = next_handoff_epoch();
    PROCESS_REGISTRY.lock().push(RegistryEntry {
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
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] launch registry helper returned\n");
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
