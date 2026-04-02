use alloc::string::String;

use super::{
    MappingRecord, Process, ProcessLifecycleState, ProcessRuntimeContractSnapshot,
    RuntimeLifecycleHooks, PAGE_ALIGN_MASK, PAGE_SIZE_BYTES_U64,
};
use crate::kernel::memory::validate_page_aligned_range;
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

        let aligned_start = seg.virtual_addr & !PAGE_ALIGN_MASK;
        let aligned_end = seg_end
            .checked_add(PAGE_ALIGN_MASK)
            .ok_or("segment address overflow")?
            & !PAGE_ALIGN_MASK;

        let bytes = aligned_end
            .checked_sub(aligned_start)
            .ok_or("invalid segment range")?;
        let pages_u64 = bytes / PAGE_SIZE_BYTES_U64;
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

fn validate_mapping_range(
    idx: usize,
    mapping: &crate::kernel::module_loader::VirtualMappingPlan,
) -> Result<usize, &'static str> {
    validate_page_aligned_range(mapping.start, mapping.end).map_err(|err| {
        let (event, message) = match err {
            crate::kernel::memory::paging::ApplyMappingError::InvalidRange => {
                ("mappings_invalid_range", "invalid mapping range")
            }
            crate::kernel::memory::paging::ApplyMappingError::MisalignedRange => {
                ("mappings_unaligned", "unaligned mapping range")
            }
            crate::kernel::memory::paging::ApplyMappingError::PageCountOverflow => {
                ("mappings_page_count_overflow", "page count overflow")
            }
            crate::kernel::memory::paging::ApplyMappingError::OutOfPhysicalMemory => {
                ("mappings_out_of_physical_memory", "out of physical memory")
            }
            crate::kernel::memory::paging::ApplyMappingError::MappingFailed => {
                ("mappings_apply_failed", "mapping failed")
            }
        };
        crate::kernel::debug_trace::record_fault("process.bind", event, Some(idx as u64));
        message
    })
}

fn publish_first_mapping_preview(
    mapping: &crate::kernel::module_loader::VirtualMappingPlan,
) {
    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mapping0_start",
        Some(mapping.start),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mapping0_end",
        Some(mapping.end),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
}

fn summarize_mapping_layout(
    mappings: &[crate::kernel::module_loader::VirtualMappingPlan],
) -> Result<(usize, usize), &'static str> {
    let mut regions = 0usize;
    let mut pages = 0usize;

    for (idx, mapping) in mappings.iter().enumerate() {
        if idx == 0 {
            publish_first_mapping_preview(mapping);
        }
        let page_count = validate_mapping_range(idx, mapping)?;
        regions = regions.checked_add(1).ok_or("region overflow")?;
        pages = pages.checked_add(page_count).ok_or("page overflow")?;
    }

    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mappings_loop_returned",
        Some(pages as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind mappings loop returned\n");

    Ok((regions, pages))
}

fn publish_mapping_counts(process: &Process, regions: usize, pages: usize) {
    process.mapped_regions.store(regions, Ordering::Relaxed);
    process.mapped_pages.store(pages, Ordering::Relaxed);
}

fn record_mapping_publish_state(regions: usize, pages: usize) {
    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mappings_published",
        Some((((regions as u64) & 0xffff_ffff) << 32) | ((pages as u64) & 0xffff_ffff)),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
}

fn record_mapping_returned_state(pages: usize) {
    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mappings_returned",
        Some(pages as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
}

#[inline(always)]
fn publish_and_record_mapping_state(process: &Process, regions: usize, pages: usize) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind mappings publish counts begin\n");
    publish_mapping_counts(process, regions, pages);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind mappings publish counts returned\n");
    record_mapping_publish_state(regions, pages);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind mappings published\n");
    record_mapping_returned_state(pages);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind mappings returned\n");
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

#[inline(always)]
pub(super) fn bind_virtual_mappings(
    process: &Process,
    mappings: &[crate::kernel::module_loader::VirtualMappingPlan],
) -> Result<(), &'static str> {
    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mappings_begin",
        Some(mappings.len() as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind mappings begin\n");
    let (regions, pages) = summarize_mapping_layout(mappings)?;
    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mappings_validated",
        Some(pages as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind mappings validated\n");
    publish_and_record_mapping_state(process, regions, pages);
    Ok(())
}

#[inline(always)]
pub fn bind_prepared_image_snapshot(
    process: &Process,
    image: &[u8],
    snapshot: &crate::kernel::module_loader::ModuleImageSnapshot,
) -> Result<(), &'static str> {
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "snapshot_begin",
        Some(snapshot.load_plan.entry),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind prepared snapshot begin\n");
    bind_module_load_plan(process, &snapshot.load_plan)?;
    bind_virtual_mappings(process, &snapshot.mappings)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] bind prepared snapshot mappings returned\n",
    );
    bind_tls_template(process, image, &snapshot.load_plan)?;
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "snapshot_returned",
        Some(snapshot.mappings.len() as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] bind prepared snapshot returned\n",
    );
    Ok(())
}

pub(super) fn allocate_user_vaddr(process: &Process, len: usize) -> Result<u64, &'static str> {
    if len == 0 {
        return Err("invalid length");
    }
    let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
    let pages = ((len as u64) + page_size - 1) / page_size;
    let bytes = pages.checked_mul(page_size).ok_or("overflow")?;
    let prev = process
        .next_mapping_hint
        .fetch_add(bytes, Ordering::Relaxed);
    let start = (prev + (page_size - 1)) & !(page_size - 1);
    Ok(start)
}

pub(super) fn register_mapping(
    process: &Process,
    map_id: u32,
    start: u64,
    end: u64,
    prot: u32,
    flags: u32,
) -> Result<(), &'static str> {
    if start >= end {
        return Err("invalid range");
    }
    let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
    if (start & (page_size - 1)) != 0 || (end & (page_size - 1)) != 0 {
        return Err("unaligned range");
    }
    let page_count = ((end - start) / page_size) as usize;

    let mut m = process.mappings.lock();
    m.push(MappingRecord {
        map_id,
        start,
        end,
        prot,
        flags,
    });
    process.mapped_regions.fetch_add(1, Ordering::Relaxed);
    process
        .mapped_pages
        .fetch_add(page_count, Ordering::Relaxed);
    Ok(())
}

pub(super) fn remove_mapping_record(process: &Process, map_id: u32) -> Option<MappingRecord> {
    let mut m = process.mappings.lock();
    if let Some(pos) = m.iter().position(|r| r.map_id == map_id) {
        let record = m.remove(pos);
        let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
        let page_count = ((record.end - record.start) / page_size) as usize;
        process.mapped_regions.fetch_sub(1, Ordering::Relaxed);
        process
            .mapped_pages
            .fetch_sub(page_count, Ordering::Relaxed);
        return Some(record);
    }
    None
}

pub(super) fn lifecycle_state(process: &Process) -> Option<ProcessLifecycleState> {
    ProcessLifecycleState::from_raw(process.lifecycle_state.load(Ordering::Relaxed))
}

pub(super) fn auxv_state(process: &Process) -> (usize, usize, usize, usize, usize, usize, usize) {
    (
        process.image_entry.load(Ordering::Relaxed),
        process.image_base.load(Ordering::Relaxed) as usize,
        process.image_phdr_addr.load(Ordering::Relaxed) as usize,
        process.image_phent_size.load(Ordering::Relaxed) as usize,
        process.image_phnum.load(Ordering::Relaxed) as usize,
        process.vdso_base.load(Ordering::Relaxed) as usize,
        process.vvar_base.load(Ordering::Relaxed) as usize,
    )
}

pub(super) fn set_exec_path(process: &Process, path: &str) {
    *process.exec_path.lock() = String::from(path);
}

pub(super) fn effective_entry(process: &Process) -> usize {
    let runtime_entry = process.runtime_entry.load(Ordering::Relaxed);
    if runtime_entry != 0 {
        runtime_entry as usize
    } else {
        process.image_entry.load(Ordering::Relaxed)
    }
}

fn publish_tls_header(process: &Process, plan: &crate::kernel::module_loader::ModuleLoadPlan) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls header publish begin\n");
    process
        .tls_mem_size
        .store(plan.tls_mem_size, Ordering::Relaxed);
    process
        .tls_align
        .store(plan.tls_align.max(1), Ordering::Relaxed);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls header publish returned\n");
}

fn clear_tls_template(process: &Process) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls bootstrap borrow begin\n");
    let tls = unsafe { process.tls_template.bootstrap_borrow_mut() };
    tls.clear();
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls bootstrap borrow returned\n");
}

fn compute_tls_file_range(
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<(usize, usize), &'static str> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls vaddr calc begin\n");
    let tls_vaddr = plan
        .tls_virtual_addr
        .checked_sub(plan.aslr_base)
        .ok_or("tls virtual address underflow")?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls vaddr calc returned\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls segment lookup begin\n");
    let segment = plan
        .segments
        .iter()
        .find(|segment| {
            let seg_end = segment
                .virtual_addr
                .checked_add(segment.mem_size)
                .unwrap_or(0);
            plan.tls_virtual_addr >= segment.virtual_addr && plan.tls_virtual_addr < seg_end
        })
        .ok_or("tls segment not covered by load segment")?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls segment lookup returned\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls delta calc begin\n");
    let delta = tls_vaddr
        .checked_sub(
            segment
                .virtual_addr
                .checked_sub(plan.aslr_base)
                .ok_or("segment base underflow")?,
        )
        .ok_or("tls segment delta underflow")?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls delta calc returned\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls file range begin\n");
    let file_offset = segment
        .file_offset
        .checked_add(delta)
        .ok_or("tls file offset overflow")? as usize;
    let file_size = usize::try_from(plan.tls_file_size).map_err(|_| "tls file size overflow")?;
    let file_end = file_offset
        .checked_add(file_size)
        .ok_or("tls file range overflow")?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls file range returned\n");
    Ok((file_offset, file_end))
}

fn build_tls_template_bytes(
    image: &[u8],
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<alloc::vec::Vec<u8>, &'static str> {
    let (file_offset, file_end) = compute_tls_file_range(plan)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls image slice begin\n");
    let bytes = image
        .get(file_offset..file_end)
        .ok_or("tls image bytes out of bounds")?;
    crate::kernel::debug_trace::record_bytes_preview("process.tls", "image_preview", bytes);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls image slice returned\n");

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls local vec begin\n");
    let reserve_len = usize::try_from(plan.tls_mem_size).map_err(|_| "tls memory size overflow")?;
    let mut next_tls = alloc::vec::Vec::with_capacity(reserve_len);
    next_tls.extend_from_slice(bytes);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls local vec returned\n");
    Ok(next_tls)
}

fn publish_tls_template_bytes(
    process: &Process,
    next_tls: alloc::vec::Vec<u8>,
    tls_mem_size: u64,
) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls bootstrap borrow begin\n");
    let tls = unsafe { process.tls_template.bootstrap_borrow_mut() };
    *tls = next_tls;
    crate::kernel::debug_trace::record_kernel_context(
        "process.bind",
        "tls_returned",
        Some(tls_mem_size),
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls bootstrap borrow returned\n");
}

#[inline(always)]
pub(super) fn bind_tls_template(
    process: &Process,
    image: &[u8],
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<(), &'static str> {
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "tls_begin",
        Some(plan.tls_mem_size),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind tls template begin\n");
    publish_tls_header(process, plan);

    if plan.tls_mem_size == 0 {
        clear_tls_template(process);
        crate::kernel::debug_trace::record_with_metadata(
            "process.bind",
            "tls_empty",
            Some(0),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Memory,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] tls empty shortcut returned\n");
        return Ok(());
    }

    let next_tls = build_tls_template_bytes(image, plan)?;

    publish_tls_template_bytes(process, next_tls, plan.tls_mem_size);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bind tls template returned\n");
    Ok(())
}

pub(super) fn runtime_contract_snapshot(process: &Process) -> ProcessRuntimeContractSnapshot {
    let hooks = process.runtime_hooks_snapshot();
    ProcessRuntimeContractSnapshot {
        image_entry: process.image_entry.load(Ordering::Relaxed),
        runtime_entry: effective_entry(process),
        runtime_fini_entry: process.runtime_fini_entry.load(Ordering::Relaxed) as usize,
        image_base: process.image_base.load(Ordering::Relaxed) as usize,
        phdr_addr: process.image_phdr_addr.load(Ordering::Relaxed) as usize,
        vdso_base: process.vdso_base.load(Ordering::Relaxed) as usize,
        vvar_base: process.vvar_base.load(Ordering::Relaxed) as usize,
        exec_path: process.exec_path_snapshot(),
        init_calls: hooks.ordered_init_calls(),
        fini_calls: hooks.ordered_fini_calls(),
    }
}

pub(super) fn append_deferred_fini_calls(process: &Process, fini_calls: &[u64]) {
    if fini_calls.is_empty() {
        return;
    }
    let mut hooks = process.runtime_hooks.lock();
    for addr in fini_calls.iter().copied().filter(|addr| *addr != 0) {
        if !hooks.deferred_fini.iter().any(|existing| *existing == addr) {
            hooks.deferred_fini.push(addr);
        }
    }
}

pub(super) fn clear_runtime_contract(process: &Process) {
    process.runtime_entry.store(0, Ordering::Relaxed);
    process.runtime_fini_entry.store(0, Ordering::Relaxed);
    process.set_runtime_hooks(RuntimeLifecycleHooks::default());
}
