use super::{Process, ProcessLifecycleState};
use core::sync::atomic::Ordering;

fn compute_total_load_plan_pages(
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<usize, &'static str> {
    let mut total_pages = 0usize;
    for seg in &plan.segments {
        let seg_end = seg
            .virtual_addr
            .checked_add(seg.mem_size)
            .ok_or("segment address overflow")?;

        let aligned_start = seg.virtual_addr & !super::super::PAGE_ALIGN_MASK;
        let aligned_end = seg_end
            .checked_add(super::super::PAGE_ALIGN_MASK)
            .ok_or("segment address overflow")?
            & !super::super::PAGE_ALIGN_MASK;

        let bytes = aligned_end
            .checked_sub(aligned_start)
            .ok_or("invalid segment range")?;
        let pages_u64 = bytes / super::super::PAGE_SIZE_BYTES_U64;
        let pages = usize::try_from(pages_u64).map_err(|_| "page count overflow")?;
        total_pages = total_pages
            .checked_add(pages)
            .ok_or("page count overflow")?;
    }
    Ok(total_pages)
}

fn publish_load_plan_state(
    process: &Process,
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
    total_pages: usize,
) {
    process
        .image_entry
        .store(plan.entry as usize, Ordering::Relaxed);
    process.runtime_entry.store(0, Ordering::Relaxed);
    process.image_base.store(plan.aslr_base, Ordering::Relaxed);
    process.image_pages.store(total_pages, Ordering::Relaxed);
    process
        .image_segments
        .store(plan.segments.len(), Ordering::Relaxed);
    process
        .tls_mem_size
        .store(plan.tls_mem_size, Ordering::Relaxed);
    process
        .tls_align
        .store(plan.tls_align.max(1), Ordering::Relaxed);
    process
        .image_phdr_addr
        .store(plan.program_header_addr, Ordering::Relaxed);
    process
        .image_phent_size
        .store(plan.program_header_entry_size as u32, Ordering::Relaxed);
    process
        .image_phnum
        .store(plan.program_headers as u32, Ordering::Relaxed);
}

fn publish_load_plan_lifecycle(process: &Process) {
    process.exec_generation.fetch_add(1, Ordering::Relaxed);
    process
        .lifecycle_state
        .store(ProcessLifecycleState::Runnable.to_u8(), Ordering::Relaxed);
    process.exit_status.store(0, Ordering::Relaxed);
}

#[inline(always)]
pub(super) fn bind_module_load_plan(
    process: &Process,
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<(), &'static str> {
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "load_plan_begin",
        Some(plan.segments.len() as u64),
        false,
    );
    let total_pages = compute_total_load_plan_pages(plan)?;
    publish_load_plan_state(process, plan, total_pages);
    publish_load_plan_lifecycle(process);
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "load_plan_returned",
        Some(total_pages as u64),
        false,
    );
    Ok(())
}
