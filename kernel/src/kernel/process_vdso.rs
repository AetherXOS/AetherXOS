use alloc::vec;
use alloc::vec::Vec;

const PAGE_SIZE_BYTES_U64: u64 = 4096;

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
        page[0..4].copy_from_slice(b"\x7FELF");
        page[4] = 2;
        page[5] = 1;
        page[6] = 1;
        page[7] = 0;
    }

    write_u64(&mut page, 16, vdso_base);
    write_u64(&mut page, 24, vvar_base);
    write_u64(&mut page, 32, page_size as u64);
    write_u64(&mut page, 40, PAGE_SIZE_BYTES_U64);
    write_u32(&mut page, 48, 1);
    write_u32(
        &mut page,
        52,
        crate::config::KernelConfig::irq_vector_base().into(),
    );
    page
}

pub(super) fn build_minimal_vvar_page(page_size: usize, entry: usize) -> Vec<u8> {
    let mut page = vec![0u8; page_size];
    write_u64(&mut page, 0, crate::hal::cpu::rdtsc());
    write_u64(&mut page, 8, PAGE_SIZE_BYTES_U64);
    write_u64(&mut page, 16, entry as u64);
    write_u64(&mut page, 24, crate::config::KernelConfig::time_slice());
    write_u64(
        &mut page,
        32,
        crate::config::KernelConfig::runtime_policy_drift_sample_interval_ticks(),
    );
    write_u64(
        &mut page,
        40,
        crate::config::KernelConfig::runtime_policy_drift_reapply_cooldown_ticks(),
    );
    page
}
