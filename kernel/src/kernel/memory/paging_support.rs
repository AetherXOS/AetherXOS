use super::paging::ApplyMappingError;

pub(crate) const PAGE_SIZE_BYTES_U64: u64 = 4096;
pub(crate) const PAGE_ALIGN_MASK: u64 = PAGE_SIZE_BYTES_U64 - 1;

pub(crate) fn validate_page_aligned_range(
    start: u64,
    end: u64,
) -> Result<usize, ApplyMappingError> {
    if start >= end {
        return Err(ApplyMappingError::InvalidRange);
    }
    if (start & PAGE_ALIGN_MASK) != 0 || (end & PAGE_ALIGN_MASK) != 0 {
        return Err(ApplyMappingError::MisalignedRange);
    }

    let page_count_u64 = (end - start) / PAGE_SIZE_BYTES_U64;
    let page_count =
        usize::try_from(page_count_u64).map_err(|_| ApplyMappingError::PageCountOverflow)?;
    if page_count == 0 {
        return Err(ApplyMappingError::InvalidRange);
    }
    Ok(page_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn validate_page_aligned_range_rejects_invalid_shapes() {
        assert_eq!(
            validate_page_aligned_range(0x2000, 0x1000),
            Err(ApplyMappingError::InvalidRange)
        );
        assert_eq!(
            validate_page_aligned_range(0x1001, 0x2000),
            Err(ApplyMappingError::MisalignedRange)
        );
        assert_eq!(
            validate_page_aligned_range(0x1000, 0x2001),
            Err(ApplyMappingError::MisalignedRange)
        );
    }

    #[test_case]
    fn validate_page_aligned_range_returns_page_count() {
        assert_eq!(validate_page_aligned_range(0x1000, 0x3000), Ok(2));
        assert_eq!(validate_page_aligned_range(0x4000, 0x5000), Ok(1));
    }

    #[test_case]
    fn validate_page_aligned_range_outputs_expected_frame_count() {
        let page_count = validate_page_aligned_range(0x1000, 0x3000).unwrap();
        assert_eq!(page_count, 2);
        assert!(page_count > 1);
    }
}
