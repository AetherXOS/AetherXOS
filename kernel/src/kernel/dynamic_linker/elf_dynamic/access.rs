/// Unsafe memory access helpers centralized to reduce scattered `unsafe` blocks.
pub unsafe fn ptr_read_unaligned_u64(addr: u64) -> u64 {
    let ptr = addr as *const u64;
    unsafe { core::ptr::read_unaligned(ptr) }
}

pub unsafe fn ptr_read_unaligned_u32(addr: u64) -> u32 {
    let ptr = addr as *const u32;
    unsafe { core::ptr::read_unaligned(ptr) }
}

pub unsafe fn ptr_write_unaligned_u64(addr: u64, val: u64) {
    let ptr = addr as *mut u64;
    unsafe { core::ptr::write_unaligned(ptr, val); }
}

pub unsafe fn ptr_copy_bytes_from_slice(dest_addr: u64, src: &[u8]) {
    unsafe { core::ptr::copy_nonoverlapping(src.as_ptr(), dest_addr as *mut u8, src.len()); }
}
