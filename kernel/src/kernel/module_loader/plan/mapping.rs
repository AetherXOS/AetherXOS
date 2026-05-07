use super::super::*;
use super::load::build_load_plan;
use super::super::{ModuleLoadPlan, VirtualMappingPlan, ModuleLoadError};
use super::super::support::{align_down, align_up, PAGE_SIZE};
use core::sync::atomic::Ordering;
use super::super::{MAP_PLAN_ATTEMPTS, MAP_PLAN_SUCCESS, MAP_PLAN_FAILURES};

pub(crate) fn build_virtual_mapping_plan_from_load_plan(
    plan: &ModuleLoadPlan,
) -> Result<Vec<VirtualMappingPlan>, ModuleLoadError> {
    let mut mappings = Vec::new();
    for segment in &plan.segments {
        let seg_end = segment
            .virtual_addr
            .checked_add(segment.mem_size)
            .ok_or(ModuleLoadError::SegmentAddressOverflow)?;

        let start = align_down(segment.virtual_addr, PAGE_SIZE);
        let end = align_up(seg_end, PAGE_SIZE).ok_or(ModuleLoadError::SegmentAddressOverflow)?;

        let zero_fill_bytes = segment.mem_size.saturating_sub(segment.file_size);
        mappings.push(VirtualMappingPlan {
            start,
            end,
            virtual_addr: segment.virtual_addr,
            mem_size: segment.mem_size,
            file_bytes: segment.file_size,
            zero_fill_bytes,
            file_offset: segment.file_offset,
        });
    }

    mappings.sort_by_key(|m| m.start);
    for pair in mappings.windows(2) {
            let prev = &pair[0];
            let next = &pair[1];
        if prev.end > next.start {
            crate::klog_warn!(
                "[LOADER] mapping overlap prev=[{:#x},{:#x}) next=[{:#x},{:#x}) file_bytes=({}, {}) zero_fill=({}, {})",
                prev.start,
                prev.end,
                next.start,
                next.end,
                prev.file_bytes,
                next.file_bytes,
                prev.zero_fill_bytes,
                next.zero_fill_bytes,
            );
            return Err(ModuleLoadError::SegmentOverlap);
        }
    }

    Ok(mappings)
}

pub fn build_virtual_mapping_plan(
    image: &[u8],
) -> Result<Vec<VirtualMappingPlan>, ModuleLoadError> {
    MAP_PLAN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let plan = match build_load_plan(image) {
        Ok(p) => p,
        Err(err) => {
            MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };

    let mappings = build_virtual_mapping_plan_from_load_plan(&plan).inspect_err(|_| {
        MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
    })?;

    MAP_PLAN_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(mappings)
}
