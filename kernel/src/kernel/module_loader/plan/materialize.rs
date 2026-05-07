use super::super::*;
use super::super::{ModuleLoadPlan, SEGMENT_MATERIALIZATION_ATTEMPTS, SEGMENT_MATERIALIZATION_FAILURES, SEGMENT_MATERIALIZED_BYTES, SEGMENT_MATERIALIZATION_SUCCESS};
use core::sync::atomic::Ordering;

pub fn materialize_load_segments(
    image: &[u8],
    load_plan: &ModuleLoadPlan,
) -> Result<u64, SegmentMaterializationError> {
    SEGMENT_MATERIALIZATION_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let mut total_bytes = 0u64;
    for segment in &load_plan.segments {
        if segment.file_size > segment.mem_size {
            SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(SegmentMaterializationError::InvalidSegmentRange);
        }

        let src_start = usize::try_from(segment.file_offset)
            .map_err(|_| SegmentMaterializationError::SegmentOutOfBounds)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let src_len = usize::try_from(segment.file_size)
            .map_err(|_| SegmentMaterializationError::SegmentOutOfBounds)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let src_end = src_start
            .checked_add(src_len)
            .ok_or(SegmentMaterializationError::SegmentOutOfBounds)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;

        if src_end > image.len() {
            SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(SegmentMaterializationError::SegmentOutOfBounds);
        }

        let dst = usize::try_from(segment.virtual_addr)
            .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let zero_fill = segment.mem_size - segment.file_size;
        let mem_size = usize::try_from(segment.mem_size)
            .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let file_size = usize::try_from(segment.file_size)
            .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;

        let _ = dst
            .checked_add(mem_size)
            .ok_or(SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;

        unsafe {
            #[cfg(target_os = "none")]
            {
                core::ptr::copy_nonoverlapping(
                    image.as_ptr().add(src_start),
                    dst as *mut u8,
                    file_size,
                );
                if zero_fill != 0 {
                    let zero_len = usize::try_from(zero_fill)
                        .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
                        .inspect_err(|_| {
                            SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
                        })?;
                    core::ptr::write_bytes((dst as *mut u8).add(file_size), 0, zero_len);
                }
            }
            #[cfg(not(target_os = "none"))]
            {
                let _ = (src_start, dst, file_size, zero_fill);
            }
        }

        total_bytes = total_bytes
            .saturating_add(segment.file_size)
            .saturating_add(zero_fill);
    }

    SEGMENT_MATERIALIZED_BYTES.fetch_add(total_bytes, Ordering::Relaxed);
    SEGMENT_MATERIALIZATION_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(total_bytes)
}
