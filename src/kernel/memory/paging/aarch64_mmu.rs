//! AArch64 4-level page table management (4KB granule, 48-bit VA).
//!
//! Supports TTBR0_EL1 (user) and TTBR1_EL1 (kernel) address spaces.
//! Implements demand paging, CoW, and TLB invalidation.

use core::ptr;

pub const PAGE_SIZE: usize = 4096;
const PAGE_SHIFT: usize = 12;
const TABLE_ENTRIES: usize = 512;
const ENTRY_ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;
const VA_BITS: usize = 48;

// Page descriptor bits
const PTE_VALID: u64 = 1 << 0;
const PTE_TABLE: u64 = 1 << 1; // For L0-L2: table descriptor
const PTE_PAGE: u64 = 1 << 1; // For L3: page descriptor
const PTE_AF: u64 = 1 << 10; // Access Flag
const PTE_AP_RW_EL1: u64 = 0b00 << 6; // R/W at EL1
const PTE_AP_RW_ALL: u64 = 0b01 << 6; // R/W at EL0+EL1
const PTE_AP_RO_EL1: u64 = 0b10 << 6; // RO at EL1
const PTE_AP_RO_ALL: u64 = 0b11 << 6; // RO at EL0+EL1
const PTE_SH_INNER: u64 = 0b11 << 8; // Inner Shareable
const PTE_UXN: u64 = 1 << 54; // Unprivileged Execute Never
const PTE_PXN: u64 = 1 << 53; // Privileged Execute Never
/// OS-available bit used for Copy-on-Write marker.
const PTE_COW: u64 = 1 << 55;
/// MAIR index for normal memory (index 0 assumed configured).
const PTE_MAIR_NORMAL: u64 = 0 << 2;

/// Frame allocator trait for AArch64 paging.
pub trait Aarch64FrameAllocator {
    /// Allocate a zeroed 4KB physical frame. Returns physical address.
    fn allocate_frame(&mut self) -> Option<u64>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aarch64MappingError {
    OutOfMemory,
    AlreadyMapped,
    NotMapped,
    MisalignedAddress,
    MappingFailed,
}

/// AArch64 Page Table flags for mapping requests.
#[derive(Debug, Clone, Copy)]
pub struct Aarch64PageFlags {
    pub writable: bool,
    pub user: bool,
    pub executable: bool,
}

impl Aarch64PageFlags {
    pub fn kernel_rw() -> Self {
        Self {
            writable: true,
            user: false,
            executable: false,
        }
    }

    pub fn user_rw() -> Self {
        Self {
            writable: true,
            user: true,
            executable: false,
        }
    }

    pub fn user_rx() -> Self {
        Self {
            writable: false,
            user: true,
            executable: true,
        }
    }

    fn to_pte_bits(self) -> u64 {
        let mut bits = PTE_VALID | PTE_PAGE | PTE_AF | PTE_SH_INNER | PTE_MAIR_NORMAL;
        if self.user {
            bits |= if self.writable {
                PTE_AP_RW_ALL
            } else {
                PTE_AP_RO_ALL
            };
        } else {
            bits |= if self.writable {
                PTE_AP_RW_EL1
            } else {
                PTE_AP_RO_EL1
            };
        }
        if !self.executable {
            bits |= PTE_UXN | PTE_PXN;
        }
        bits
    }
}

/// AArch64 Virtual Memory Manager.
pub struct Aarch64PageManager {
    /// Physical address of the L0 (PGD) table — loaded into TTBR0 or TTBR1.
    root_table_phys: u64,
    /// Virtual offset where all physical memory is identity/offset-mapped.
    phys_offset: u64,
}

impl Aarch64PageManager {
    /// Create a new page manager.
    /// - `root_table_phys`: physical address of the root page table
    /// - `phys_offset`: virtual address offset for physical memory access
    pub unsafe fn new(root_table_phys: u64, phys_offset: u64) -> Self {
        Self {
            root_table_phys,
            phys_offset,
        }
    }

    /// Get the physical address of the root table (for loading into TTBR).
    pub fn root_phys(&self) -> u64 {
        self.root_table_phys
    }

    fn phys_to_virt(&self, phys: u64) -> *mut u64 {
        (self.phys_offset + phys) as *mut u64
    }

    /// Walk the page table hierarchy, optionally allocating intermediate tables.
    fn walk_table(
        &mut self,
        va: u64,
        allocate: bool,
        alloc: &mut impl Aarch64FrameAllocator,
    ) -> Result<*mut u64, Aarch64MappingError> {
        let indices = [
            ((va >> 39) & 0x1FF) as usize, // L0
            ((va >> 30) & 0x1FF) as usize, // L1
            ((va >> 21) & 0x1FF) as usize, // L2
            ((va >> 12) & 0x1FF) as usize, // L3
        ];

        let mut table_phys = self.root_table_phys;

        // Walk L0 -> L1 -> L2, each time resolving or creating the next table
        for level in 0..3 {
            let table_ptr = self.phys_to_virt(table_phys);
            let entry_ptr = unsafe { table_ptr.add(indices[level]) };
            let entry = unsafe { ptr::read_volatile(entry_ptr) };

            if entry & PTE_VALID != 0 {
                // Existing table descriptor
                table_phys = entry & ENTRY_ADDR_MASK;
            } else if allocate {
                // Allocate a new intermediate table
                let new_table = alloc
                    .allocate_frame()
                    .ok_or(Aarch64MappingError::OutOfMemory)?;
                // Zero the new table
                unsafe {
                    ptr::write_bytes(self.phys_to_virt(new_table) as *mut u8, 0, PAGE_SIZE);
                }
                let desc = new_table | PTE_VALID | PTE_TABLE;
                unsafe {
                    ptr::write_volatile(entry_ptr, desc);
                }
                table_phys = new_table;
            } else {
                return Err(Aarch64MappingError::NotMapped);
            }
        }

        // Return pointer to the L3 entry
        let l3_ptr = self.phys_to_virt(table_phys);
        Ok(unsafe { l3_ptr.add(indices[3]) })
    }

    /// Map a single 4KB page.
    pub fn map_page(
        &mut self,
        va: u64,
        pa: u64,
        flags: Aarch64PageFlags,
        alloc: &mut impl Aarch64FrameAllocator,
    ) -> Result<(), Aarch64MappingError> {
        if va & 0xFFF != 0 || pa & 0xFFF != 0 {
            return Err(Aarch64MappingError::MisalignedAddress);
        }

        let entry_ptr = self.walk_table(va, true, alloc)?;
        let existing = unsafe { ptr::read_volatile(entry_ptr) };
        if existing & PTE_VALID != 0 {
            return Err(Aarch64MappingError::AlreadyMapped);
        }

        let desc = pa | flags.to_pte_bits();
        unsafe {
            ptr::write_volatile(entry_ptr, desc);
        }
        tlbi_va(va);
        Ok(())
    }

    /// Unmap a single 4KB page. Returns the physical address of the unmapped frame.
    pub fn unmap_page(&mut self, va: u64) -> Result<u64, Aarch64MappingError> {
        if va & 0xFFF != 0 {
            return Err(Aarch64MappingError::MisalignedAddress);
        }

        struct NullAlloc;
        impl Aarch64FrameAllocator for NullAlloc {
            fn allocate_frame(&mut self) -> Option<u64> {
                None
            }
        }
        let entry_ptr = self.walk_table(va, false, &mut NullAlloc)?;
        let entry = unsafe { ptr::read_volatile(entry_ptr) };
        if entry & PTE_VALID == 0 {
            return Err(Aarch64MappingError::NotMapped);
        }

        let phys = entry & ENTRY_ADDR_MASK;
        unsafe {
            ptr::write_volatile(entry_ptr, 0);
        }
        tlbi_va(va);
        Ok(phys)
    }

    /// Handle a page fault with Copy-on-Write support.
    pub fn handle_page_fault(
        &mut self,
        va: u64,
        alloc: &mut impl Aarch64FrameAllocator,
    ) -> Result<(), Aarch64MappingError> {
        struct NullAlloc;
        impl Aarch64FrameAllocator for NullAlloc {
            fn allocate_frame(&mut self) -> Option<u64> {
                None
            }
        }

        // Try to read existing mapping
        let entry_ptr = match self.walk_table(va, false, &mut NullAlloc) {
            Ok(p) => p,
            Err(Aarch64MappingError::NotMapped) => {
                // Demand paging: allocate and map a new frame
                let frame = alloc
                    .allocate_frame()
                    .ok_or(Aarch64MappingError::OutOfMemory)?;
                unsafe {
                    ptr::write_bytes(self.phys_to_virt(frame) as *mut u8, 0, PAGE_SIZE);
                }
                return self.map_page(va, frame, Aarch64PageFlags::user_rw(), alloc);
            }
            Err(e) => return Err(e),
        };

        let entry = unsafe { ptr::read_volatile(entry_ptr) };
        if entry & PTE_VALID == 0 {
            // Not valid — demand page
            let frame = alloc
                .allocate_frame()
                .ok_or(Aarch64MappingError::OutOfMemory)?;
            unsafe {
                ptr::write_bytes(self.phys_to_virt(frame) as *mut u8, 0, PAGE_SIZE);
            }
            return self.map_page(va, frame, Aarch64PageFlags::user_rw(), alloc);
        }

        // Check for CoW
        let is_cow = (entry & PTE_COW) != 0;
        if !is_cow {
            return Err(Aarch64MappingError::AlreadyMapped);
        }

        // CoW: copy old frame to new frame
        let old_phys = entry & ENTRY_ADDR_MASK;
        let new_frame = alloc
            .allocate_frame()
            .ok_or(Aarch64MappingError::OutOfMemory)?;

        unsafe {
            ptr::copy_nonoverlapping(
                self.phys_to_virt(old_phys) as *const u8,
                self.phys_to_virt(new_frame) as *mut u8,
                PAGE_SIZE,
            );
        }

        // Remap as writable, clear CoW bit
        let new_entry = new_frame
            | PTE_VALID
            | PTE_PAGE
            | PTE_AF
            | PTE_SH_INNER
            | PTE_MAIR_NORMAL
            | PTE_AP_RW_ALL;
        unsafe {
            ptr::write_volatile(entry_ptr, new_entry);
        }
        tlbi_va(va);
        Ok(())
    }

    /// Mark a range of pages as Copy-on-Write (read-only + COW bit).
    pub fn mark_cow_range(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<usize, Aarch64MappingError> {
        if start & 0xFFF != 0 || end & 0xFFF != 0 {
            return Err(Aarch64MappingError::MisalignedAddress);
        }

        struct NullAlloc;
        impl Aarch64FrameAllocator for NullAlloc {
            fn allocate_frame(&mut self) -> Option<u64> {
                None
            }
        }

        let mut count = 0usize;
        let mut va = start;
        while va < end {
            if let Ok(entry_ptr) = self.walk_table(va, false, &mut NullAlloc) {
                let entry = unsafe { ptr::read_volatile(entry_ptr) };
                if entry & PTE_VALID != 0 {
                    // Set COW bit, make read-only
                    let new_entry = (entry | PTE_COW | PTE_AP_RO_ALL) & !PTE_AP_RW_ALL;
                    unsafe {
                        ptr::write_volatile(entry_ptr, new_entry);
                    }
                    tlbi_va(va);
                    count += 1;
                }
            }
            va += PAGE_SIZE as u64;
        }

        Ok(count)
    }

    /// Map a virtual range using a contiguous physical region.
    pub fn map_range(
        &mut self,
        va_start: u64,
        pa_start: u64,
        page_count: usize,
        flags: Aarch64PageFlags,
        alloc: &mut impl Aarch64FrameAllocator,
    ) -> Result<(), Aarch64MappingError> {
        for i in 0..page_count {
            let offset = (i as u64) * PAGE_SIZE as u64;
            self.map_page(va_start + offset, pa_start + offset, flags, alloc)?;
        }
        Ok(())
    }
}

/// TLB invalidation for a single virtual address (EL1).
#[inline(always)]
fn tlbi_va(va: u64) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let va_shifted = va >> 12;
        core::arch::asm!(
            "tlbi vale1is, {va}",
            "dsb ish",
            "isb",
            va = in(reg) va_shifted,
        );
    }
}

/// Full TLB invalidation (all entries, EL1).
#[cfg(target_arch = "aarch64")]
pub fn tlbi_all() {
    unsafe {
        core::arch::asm!("tlbi vmalle1is", "dsb ish", "isb",);
    }
}

/// Load a page table address into TTBR0_EL1 (user space).
#[cfg(target_arch = "aarch64")]
pub unsafe fn set_ttbr0(phys: u64) {
    unsafe {
        core::arch::asm!(
            "msr ttbr0_el1, {phys}",
            "isb",
            phys = in(reg) phys,
        );
    }
}

/// Load a page table address into TTBR1_EL1 (kernel space).
#[cfg(target_arch = "aarch64")]
pub unsafe fn set_ttbr1(phys: u64) {
    unsafe {
        core::arch::asm!(
            "msr ttbr1_el1, {phys}",
            "isb",
            phys = in(reg) phys,
        );
    }
}

/// Configure TCR_EL1 for 48-bit VA, 4KB granule.
#[cfg(target_arch = "aarch64")]
pub unsafe fn configure_tcr() {
    // T0SZ = T1SZ = 16 (48-bit VA)
    // TG0 = TG1 = 4KB granule (0b00 for TG0, 0b10 for TG1)
    // IPS = 48-bit PA (0b101)
    // SH0 = SH1 = Inner Shareable (0b11)
    // ORGN/IRGN = Write-Back Allocate (0b01)
    let tcr: u64 = (16 << 0)     // T0SZ
        | (16 << 16)             // T1SZ
        | (0b00 << 14)           // TG0 = 4KB
        | (0b10 << 30)           // TG1 = 4KB
        | (0b101u64 << 32)       // IPS = 48-bit
        | (0b11 << 12)           // SH0 = Inner Shareable
        | (0b11 << 28)           // SH1 = Inner Shareable
        | (0b01 << 10)           // ORGN0
        | (0b01 << 8)            // IRGN0
        | (0b01 << 26)           // ORGN1
        | (0b01 << 24); // IRGN1
    unsafe {
        core::arch::asm!(
            "msr tcr_el1, {tcr}",
            "isb",
            tcr = in(reg) tcr,
        );
    }
}
