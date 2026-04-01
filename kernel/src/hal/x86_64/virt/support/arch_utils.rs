#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub(crate) fn rdmsr(msr: u32) -> u64 {
    let high: u32;
    let low: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("edx") high,
            out("eax") low,
            options(nostack, nomem)
        );
    }
    ((high as u64) << 32) | low as u64
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub(crate) fn wrmsr(msr: u32, value: u64) {
    let high = (value >> 32) as u32;
    let low = value as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("edx") high,
            in("eax") low,
            options(nostack, nomem)
        );
    }
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn virt_to_phys(addr: usize) -> Option<u64> {
    let offset = crate::hal::x86_64::hhdm_offset()? as usize;
    if addr >= offset {
        Some((addr - offset) as u64)
    } else {
        Some(addr as u64)
    }
}
