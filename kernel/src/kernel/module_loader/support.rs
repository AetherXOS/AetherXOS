pub(super) const PAGE_SIZE: u64 = 4096;
pub(super) const ELF_HEADER_MIN_BYTES: usize = 64;
const FNV1A_OFFSET_BASIS_64: u64 = 1469598103934665603;
const FNV1A_PRIME_64: u64 = 1099511628211;

#[inline(always)]
pub(super) fn current_target_elf_machine() -> xmas_elf::header::Machine {
    #[cfg(target_arch = "x86_64")]
    {
        xmas_elf::header::Machine::X86_64
    }

    #[cfg(target_arch = "aarch64")]
    {
        xmas_elf::header::Machine::AArch64
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        xmas_elf::header::Machine::None_
    }
}

#[inline(always)]
pub(super) fn elf_machine_matches_target(machine: xmas_elf::header::Machine) -> bool {
    machine == current_target_elf_machine()
}

#[inline(always)]
pub(super) fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}

#[inline(always)]
pub(super) fn align_up(value: u64, align: u64) -> Option<u64> {
    value.checked_add(align - 1).map(|v| v & !(align - 1))
}

pub(super) fn checked_table_end(
    offset: usize,
    count: usize,
    entsize: usize,
    image_len: usize,
) -> Option<usize> {
    let bytes = count.checked_mul(entsize)?;
    let end = offset.checked_add(bytes)?;
    if end <= image_len {
        Some(end)
    } else {
        None
    }
}

pub(super) fn image_fingerprint(image: &[u8]) -> u64 {
    let mut hash = FNV1A_OFFSET_BASIS_64;
    for b in image {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(FNV1A_PRIME_64);
    }
    hash
}

pub(super) fn segment_range_fits_image(file_offset: u64, file_size: u64, image_len: usize) -> bool {
    let start = usize::try_from(file_offset).ok();
    let len = usize::try_from(file_size).ok();
    start
        .and_then(|s| len.and_then(|l| s.checked_add(l)))
        .is_some_and(|e| e <= image_len)
}

pub(super) fn entry_in_segments(entry: u64, segments: &[(u64, u64)]) -> bool {
    segments.iter().any(|(start, mem_size)| {
        start
            .checked_add(*mem_size)
            .is_some_and(|end| entry >= *start && entry < end)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn align_helpers_round_and_clip_consistently() {
        assert_eq!(align_down(0x1234, PAGE_SIZE), 0x1000);
        assert_eq!(align_up(0x1001, PAGE_SIZE), Some(0x2000));
        assert_eq!(align_up(u64::MAX, PAGE_SIZE), None);
    }

    #[test_case]
    fn checked_table_end_detects_overflow_and_bounds() {
        assert_eq!(checked_table_end(10, 4, 8, 64), Some(42));
        assert_eq!(checked_table_end(60, 2, 8, 64), None);
        assert_eq!(checked_table_end(usize::MAX, 2, 8, usize::MAX), None);
    }

    #[test_case]
    fn fingerprint_is_stable_and_sensitive_to_content() {
        let a = image_fingerprint(b"elf-a");
        let b = image_fingerprint(b"elf-b");
        assert_eq!(a, image_fingerprint(b"elf-a"));
        assert_ne!(a, b);
    }

    #[test_case]
    fn segment_range_and_entry_helpers_reject_oob_layouts() {
        assert!(segment_range_fits_image(4, 8, 32));
        assert!(!segment_range_fits_image(u64::MAX, 8, 32));
        assert!(!segment_range_fits_image(30, 8, 32));

        let segments = [(0x1000, 0x200), (0x2000, 0x300)];
        assert!(entry_in_segments(0x1100, &segments));
        assert!(entry_in_segments(0x2200, &segments));
        assert!(!entry_in_segments(0x3000, &segments));
    }

    #[test_case]
    fn current_target_machine_matches_supported_arches() {
        #[cfg(target_arch = "x86_64")]
        assert_eq!(
            current_target_elf_machine(),
            xmas_elf::header::Machine::X86_64
        );

        #[cfg(target_arch = "aarch64")]
        assert_eq!(
            current_target_elf_machine(),
            xmas_elf::header::Machine::AArch64
        );
    }
}
