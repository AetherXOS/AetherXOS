use alloc::vec;
use alloc::vec::Vec;

const PAGE_SIZE_BYTES_U64: u64 = 4096;
const VDSO_MAGIC_OFFSET: usize = 0;
const VDSO_BASE_OFFSET: usize = 16;
const VDSO_VVAR_BASE_OFFSET: usize = 24;
const VDSO_PAGE_SIZE_OFFSET: usize = 32;
const VDSO_MIN_PAGE_SIZE_OFFSET: usize = 40;
const VDSO_IRQ_VECTOR_OFFSET: usize = 52;

const VVAR_RDTSC_OFFSET: usize = 0;
const VVAR_PAGE_SIZE_OFFSET: usize = 8;
const VVAR_ENTRY_OFFSET: usize = 16;
const VVAR_TIME_SLICE_OFFSET: usize = 24;
const VVAR_POLICY_SAMPLE_OFFSET: usize = 32;
const VVAR_POLICY_COOLDOWN_OFFSET: usize = 40;

#[inline(always)]
fn write_u32(buf: &mut [u8], off: usize, value: u32) {
    if off.saturating_add(4) <= buf.len() {
        buf[off..off + 4].copy_from_slice(&value.to_le_bytes());
    }
}

#[inline(always)]
fn write_u64(buf: &mut [u8], off: usize, value: u64) {
    if off.saturating_add(8) <= buf.len() {
        buf[off..off + 8].copy_from_slice(&value.to_le_bytes());
    }
}

pub(super) fn build_minimal_vdso_page(page_size: usize, vdso_base: u64, vvar_base: u64) -> Vec<u8> {
    let mut page = vec![0u8; page_size];
    if page.len() >= 16 {
        page[VDSO_MAGIC_OFFSET..4].copy_from_slice(b"\x7FELF");
        page[4] = 2;
        page[5] = 1;
        page[6] = 1;
        page[7] = 0;
    }

    write_u64(&mut page, VDSO_BASE_OFFSET, vdso_base);
    write_u64(&mut page, VDSO_VVAR_BASE_OFFSET, vvar_base);
    write_u64(&mut page, VDSO_PAGE_SIZE_OFFSET, page_size as u64);
    write_u64(&mut page, VDSO_MIN_PAGE_SIZE_OFFSET, PAGE_SIZE_BYTES_U64);
    write_u32(&mut page, 48, 1);
    write_u32(&mut page, VDSO_IRQ_VECTOR_OFFSET, crate::config::KernelConfig::irq_vector_base().into());
    page
}

pub(super) fn build_minimal_vvar_page(page_size: usize, entry: usize) -> Vec<u8> {
    let mut page = vec![0u8; page_size];
    write_u64(&mut page, VVAR_RDTSC_OFFSET, crate::hal::cpu::rdtsc());
    write_u64(&mut page, VVAR_PAGE_SIZE_OFFSET, PAGE_SIZE_BYTES_U64);
    write_u64(&mut page, VVAR_ENTRY_OFFSET, entry as u64);
    write_u64(&mut page, VVAR_TIME_SLICE_OFFSET, crate::config::KernelConfig::time_slice());
    write_u64(
        &mut page,
        VVAR_POLICY_SAMPLE_OFFSET,
        crate::config::KernelConfig::runtime_policy_drift_sample_interval_ticks(),
    );
    write_u64(
        &mut page,
        VVAR_POLICY_COOLDOWN_OFFSET,
        crate::config::KernelConfig::runtime_policy_drift_reapply_cooldown_ticks(),
    );
    page
}

pub(super) fn validate_linux_runtime_pages(
    vdso_page: &[u8],
    vvar_page: &[u8],
    vdso_base: u64,
    vvar_base: u64,
    entry: usize,
) -> bool {
    if vdso_page.len() < 56 || vvar_page.len() < 48 {
        return false;
    }
    if vdso_base == 0 || vvar_base == 0 || vdso_base == vvar_base {
        return false;
    }
    if vdso_base & 0xfff != 0 || vvar_base & 0xfff != 0 {
        return false;
    }

    if vdso_page.get(0..4) != Some(b"\x7FELF") {
        return false;
    }
    let read_u64 = |page: &[u8], off: usize| -> Option<u64> {
        let raw: [u8; 8] = page.get(off..off + 8)?.try_into().ok()?;
        Some(u64::from_le_bytes(raw))
    };
    let read_u32 = |page: &[u8], off: usize| -> Option<u32> {
        let raw: [u8; 4] = page.get(off..off + 4)?.try_into().ok()?;
        Some(u32::from_le_bytes(raw))
    };

    read_u64(vdso_page, VDSO_BASE_OFFSET) == Some(vdso_base)
        && read_u64(vdso_page, VDSO_VVAR_BASE_OFFSET) == Some(vvar_base)
        && read_u64(vdso_page, VDSO_PAGE_SIZE_OFFSET) == Some(vdso_page.len() as u64)
        && read_u64(vdso_page, VDSO_MIN_PAGE_SIZE_OFFSET) == Some(PAGE_SIZE_BYTES_U64)
        && read_u32(vdso_page, 48) == Some(1)
        && read_u32(vdso_page, VDSO_IRQ_VECTOR_OFFSET).is_some()
        && read_u64(vvar_page, VVAR_PAGE_SIZE_OFFSET) == Some(PAGE_SIZE_BYTES_U64)
        && read_u64(vvar_page, VVAR_ENTRY_OFFSET) == Some(entry as u64)
        && read_u64(vvar_page, VVAR_TIME_SLICE_OFFSET)
            == Some(crate::config::KernelConfig::time_slice())
        && read_u64(vvar_page, VVAR_POLICY_SAMPLE_OFFSET)
            == Some(crate::config::KernelConfig::runtime_policy_drift_sample_interval_ticks())
        && read_u64(vvar_page, VVAR_POLICY_COOLDOWN_OFFSET)
            == Some(crate::config::KernelConfig::runtime_policy_drift_reapply_cooldown_ticks())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn validate_linux_runtime_pages_accepts_matching_pages() {
        let vdso_base = 0x7000_0000;
        let vvar_base = 0x7000_1000;
        let vdso_page = build_minimal_vdso_page(4096, vdso_base, vvar_base);
        let vvar_page = build_minimal_vvar_page(4096, 0x401000);

        assert!(validate_linux_runtime_pages(
            &vdso_page,
            &vvar_page,
            vdso_base,
            vvar_base,
            0x401000
        ));
    }

    #[test_case]
    fn validate_linux_runtime_pages_rejects_misaligned_or_corrupted_pages() {
        let vdso_base = 0x7000_0000;
        let vvar_base = 0x7000_1000;
        let mut vdso_page = build_minimal_vdso_page(4096, vdso_base, vvar_base);
        let vvar_page = build_minimal_vvar_page(4096, 0x401000);
        vdso_page[VDSO_VVAR_BASE_OFFSET] ^= 0x1;

        assert!(!validate_linux_runtime_pages(
            &vdso_page,
            &vvar_page,
            vdso_base,
            vvar_base,
            0x401000
        ));
    }
}
