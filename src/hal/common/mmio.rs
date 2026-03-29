#[inline(always)]
pub fn phys_to_hhdm_ptr<T>(phys: u64) -> Option<*mut T> {
    let offset = super::boot::hhdm_offset()?;
    let virt = phys.checked_add(offset)?;
    Some(virt as *mut T)
}

#[inline(always)]
pub fn read_phys_u32(phys: u64) -> Option<u32> {
    let ptr = phys_to_hhdm_ptr::<u32>(phys)?;
    // SAFETY: The caller provides a physical MMIO address backed by an HHDM mapping.
    Some(unsafe { core::ptr::read_volatile(ptr) })
}

#[inline(always)]
pub fn write_phys_u32(phys: u64, value: u32) -> bool {
    if let Some(ptr) = phys_to_hhdm_ptr::<u32>(phys) {
        // SAFETY: The caller provides a physical MMIO address backed by an HHDM mapping.
        unsafe { core::ptr::write_volatile(ptr, value) };
        true
    } else {
        false
    }
}

#[inline(always)]
pub fn read_phys_u64(phys: u64) -> Option<u64> {
    let ptr = phys_to_hhdm_ptr::<u64>(phys)?;
    // SAFETY: The caller provides a physical MMIO address backed by an HHDM mapping.
    Some(unsafe { core::ptr::read_volatile(ptr) })
}

#[inline(always)]
pub fn write_phys_u64(phys: u64, value: u64) -> bool {
    if let Some(ptr) = phys_to_hhdm_ptr::<u64>(phys) {
        // SAFETY: The caller provides a physical MMIO address backed by an HHDM mapping.
        unsafe { core::ptr::write_volatile(ptr, value) };
        true
    } else {
        false
    }
}

#[inline(always)]
pub unsafe fn read_virt_u32(addr: u64) -> u32 {
    let ptr = addr as *const u32;
    // SAFETY: Caller guarantees this virtual address points to valid MMIO.
    unsafe { core::ptr::read_volatile(ptr) }
}

#[inline(always)]
pub unsafe fn write_virt_u32(addr: u64, value: u32) {
    let ptr = addr as *mut u32;
    // SAFETY: Caller guarantees this virtual address points to valid MMIO.
    unsafe { core::ptr::write_volatile(ptr, value) };
}

#[inline(always)]
pub fn virt_to_phys(addr: usize) -> Option<u64> {
    let offset = super::boot::hhdm_offset()? as usize;
    if addr >= offset {
        Some((addr - offset) as u64)
    } else {
        Some(addr as u64)
    }
}
