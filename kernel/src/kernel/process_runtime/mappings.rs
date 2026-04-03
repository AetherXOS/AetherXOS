use super::{trace, Process};
use core::sync::atomic::Ordering;

use crate::kernel::memory::validate_page_aligned_range;

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

fn publish_first_mapping_preview(mapping: &crate::kernel::module_loader::VirtualMappingPlan) {
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
    trace::early_serial("[EARLY SERIAL] bind mappings loop returned\n");

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
    trace::early_serial("[EARLY SERIAL] bind mappings publish counts begin\n");
    publish_mapping_counts(process, regions, pages);
    trace::early_serial("[EARLY SERIAL] bind mappings publish counts returned\n");
    record_mapping_publish_state(regions, pages);
    trace::early_serial("[EARLY SERIAL] bind mappings published\n");
    record_mapping_returned_state(pages);
    trace::early_serial("[EARLY SERIAL] bind mappings returned\n");
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
    trace::early_serial("[EARLY SERIAL] bind mappings begin\n");
    let (regions, pages) = summarize_mapping_layout(mappings)?;
    crate::kernel::debug_trace::record_with_metadata(
        "process.bind",
        "mappings_validated",
        Some(pages as u64),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Memory,
    );
    trace::early_serial("[EARLY SERIAL] bind mappings validated\n");
    publish_and_record_mapping_state(process, regions, pages);
    Ok(())
}
