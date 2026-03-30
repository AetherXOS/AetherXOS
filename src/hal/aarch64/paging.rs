//! AArch64 4-level page table management (4KB granule, 48-bit VA).
//! Standardizes Paging behavior into the HAL.

use core::ptr;
use crate::kernel::bit_utils::paging as bits;
use crate::interfaces::memory::page_flags as common_bits;

pub const PAGE_SIZE: usize = bits::PAGE_SIZE;
const ADDR_MASK: u64 = bits::ENTRY_ADDR_MASK;

define_flags!(pub struct ArchPageFlags: u64 {
    VALID       = 1 << 0,
    TABLE       = 1 << 1,
    PAGE        = 1 << 1,
    AF          = 1 << 10,
    AP_RW_ALL   = 0b01 << 6,
    AP_RO_ALL   = 0b11 << 6,
    SH_INNER    = 0b11 << 8,
    UXN         = 1 << 54,
    PXN         = 1 << 53,
    COW         = 1 << 55,
    MAIR_NORMAL = 0 << 2,
});

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<u64>;
}

fn convert_flags(flags: u32) -> ArchPageFlags {
    let mut bits = ArchPageFlags::VALID | ArchPageFlags::PAGE | ArchPageFlags::AF | 
                   ArchPageFlags::SH_INNER | ArchPageFlags::MAIR_NORMAL;
    if (flags & common_bits::USER) != 0 {
        bits |= if (flags & common_bits::WRITABLE) != 0 { ArchPageFlags::AP_RW_ALL } else { ArchPageFlags::AP_RO_ALL };
    }
    if (flags & common_bits::NO_EXECUTE) != 0 {
        bits |= ArchPageFlags::UXN | ArchPageFlags::PXN;
    }
    if (flags & common_bits::COW) != 0 { bits |= ArchPageFlags::COW; }
    bits
}

pub struct PageManager {
    root_table_phys: u64,
    phys_offset: u64,
}

impl PageManager {
    pub unsafe fn new(root_table_phys: u64, phys_offset: u64) -> Self {
        Self { root_table_phys, phys_offset }
    }

    fn phys_to_virt(&self, phys: u64) -> *mut u64 {
        (self.phys_offset + phys) as *mut u64
    }

    fn walk_table(
        &mut self,
        va: u64,
        allocate: bool,
        alloc: &mut impl FrameAllocator,
    ) -> Result<*mut u64, &'static str> {
        let indices = bits::get_indices(va);
        let mut table_phys = self.root_table_phys;

        for level in 0..3 {
            let table_ptr = self.phys_to_virt(table_phys);
            let entry_ptr = unsafe { table_ptr.add(indices[level]) };
            let entry = unsafe { ptr::read_volatile(entry_ptr) };

            if (entry & ArchPageFlags::VALID.bits()) != 0 {
                table_phys = entry & ADDR_MASK;
            } else if allocate {
                let new_table = alloc.allocate_frame().ok_or("Out of memory")?;
                unsafe { ptr::write_bytes(self.phys_to_virt(new_table) as *mut u8, 0, PAGE_SIZE); }
                let desc = new_table | (ArchPageFlags::VALID | ArchPageFlags::TABLE).bits();
                unsafe { ptr::write_volatile(entry_ptr, desc); }
                table_phys = new_table;
            } else { return Err("Not mapped"); }
        }

        let l3_ptr = self.phys_to_virt(table_phys);
        Ok(unsafe { l3_ptr.add(indices[3]) })
    }

    pub fn map_page(
        &mut self,
        va: u64,
        pa: u64,
        flags: u32,
        alloc: &mut impl FrameAllocator,
    ) -> Result<(), &'static str> {
        let entry_ptr = self.walk_table(va, true, alloc)?;
        let desc = (pa & ADDR_MASK) | convert_flags(flags).bits();
        unsafe { ptr::write_volatile(entry_ptr, desc); }
        tlbi_va(va);
        Ok(())
    }

    pub fn handle_page_fault(
        &mut self,
        va: u64,
        alloc: &mut impl FrameAllocator,
    ) -> Result<(), &'static str> {
        let entry_ptr = match self.walk_table(va, false, &mut NullAlloc) {
            Ok(p) => p,
            Err(_) => {
                let frame = alloc.allocate_frame().ok_or("Out of memory")?;
                unsafe { ptr::write_bytes(self.phys_to_virt(frame) as *mut u8, 0, PAGE_SIZE); }
                let flags = common_bits::PRESENT | common_bits::WRITABLE | common_bits::USER;
                return self.map_page(va, frame, flags, alloc);
            }
        };

        let entry = unsafe { ptr::read_volatile(entry_ptr) };
        if (entry & ArchPageFlags::COW.bits()) != 0 {
            let old_phys = entry & ADDR_MASK;
            let new_frame = alloc.allocate_frame().ok_or("Out of memory")?;
            unsafe {
                ptr::copy_nonoverlapping(self.phys_to_virt(old_phys) as *const u8, self.phys_to_virt(new_frame) as *mut u8, PAGE_SIZE);
                let mut f = convert_flags(common_bits::PRESENT | common_bits::WRITABLE | common_bits::USER);
                f.remove(ArchPageFlags::COW);
                let new_entry = (new_frame & ADDR_MASK) | f.bits();
                ptr::write_volatile(entry_ptr, new_entry);
            }
            tlbi_va(va);
            return Ok(());
        }
        Err("Already mapped")
    }
}

struct NullAlloc;
impl FrameAllocator for NullAlloc {
    fn allocate_frame(&mut self) -> Option<u64> { None }
}

#[inline(always)]
fn tlbi_va(va: u64) {
    unsafe {
        let va_shifted = va >> 12;
        core::arch::asm!("tlbi vale1is, {va}", "dsb ish", "isb", va = in(reg) va_shifted);
    }
}
