//! Platform-agnostic Virtual Memory Manager (VMM).
//! Delegated to the Hardware Abstraction Layer (HAL) for platform-specifics.

use crate::kernel::bit_utils::paging as bits;
#[cfg(target_arch = "x86_64")]
use x86_64::registers::control::Cr3;
pub use crate::hal::paging::PageManager;

#[derive(Debug, Clone, Copy)]
pub struct AppliedMappingStats {
    pub regions: usize,
    pub pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyMappingError {
    MisalignedRange,
    InvalidRange,
    PageCountOverflow,
    OutOfPhysicalMemory,
    MappingFailed,
}

#[cfg(target_arch = "x86_64")]
impl PageManager {
    /// Internal helper for bulk mapping virtual ranges.
    fn map_range<F>(
        &mut self,
        start: u64,
        end: u64,
        flags: u32,
        alloc: &mut crate::hal::paging::PageAllocWrapper,
        mut get_phys: F,
    ) -> Result<usize, ApplyMappingError>
    where
        F: FnMut(usize, &mut crate::hal::paging::PageAllocWrapper) -> Result<u64, ApplyMappingError>,
    {
        if (start & (bits::PAGE_SIZE as u64 - 1)) != 0 || (end & (bits::PAGE_SIZE as u64 - 1)) != 0 {
            return Err(ApplyMappingError::MisalignedRange);
        }

        let mut pages = 0usize;
        let count = ((end - start) / bits::PAGE_SIZE as u64) as usize;
        for i in 0..count {
            let va = start + (i as u64 * bits::PAGE_SIZE as u64);
            let pa = get_phys(i, alloc)?;
            self.map_page(va, pa, flags, alloc).map_err(|_| ApplyMappingError::MappingFailed)?;
            pages += 1;
        }
        Ok(pages)
    }

    /// Map a 2MB HugePage.
    pub fn map_huge_page(
        &mut self,
        va: u64,
        pa: u64,
        flags: u32,
        alloc: &mut crate::hal::paging::PageAllocWrapper,
    ) -> Result<(), ApplyMappingError> {
        // Platform-specific huge page mapping logic (e.g. set HUGO bit in PDE)
        // Here we simulate it or delegate to HAL.
        self.map_page_2mb(va, pa, flags, alloc).map_err(|_| ApplyMappingError::MappingFailed)
    }

    pub fn apply_virtual_mapping_plan(
        &mut self,
        mappings: &[crate::kernel::module_loader::VirtualMappingPlan],
        flags: u32,
        alloc: &mut crate::hal::paging::PageAllocWrapper,
    ) -> Result<AppliedMappingStats, ApplyMappingError> {
        let mut regions = 0usize;
        let mut pages = 0usize;

        for mapping in mappings {
            let size = mapping.end - mapping.start;
            // Use HugePages (2MB) if aligned and size >= 2MB
            if mapping.start % 0x200_000 == 0 && size >= 0x200_000 {
                let huge_count = (size / 0x200_000) as usize;
                for i in 0..huge_count {
                    let va = mapping.start + (i as u64 * 0x200_000);
                    let pa = alloc.allocate_huge_frame().map(|f| f.start_address().as_u64()).ok_or(ApplyMappingError::OutOfPhysicalMemory)?;
                    self.map_huge_page(va, pa, flags, alloc)?;
                    pages += 512; // One 2MB page = 512 4KB pages
                }
                // Map remaining 4KB tails
                let tail_start = mapping.start + (huge_count as u64 * 0x200_000);
                if tail_start < mapping.end {
                    pages += self.map_range(tail_start, mapping.end, flags, alloc, |_, a| {
                        a.allocate_frame().map(|f| f.start_address().as_u64()).ok_or(ApplyMappingError::OutOfPhysicalMemory)
                    })?;
                }
            } else {
                pages += self.map_range(mapping.start, mapping.end, flags, alloc, |_, a: &mut crate::hal::paging::PageAllocWrapper| {
                    a.allocate_frame().map(|f| f.start_address().as_u64()).ok_or(ApplyMappingError::OutOfPhysicalMemory)
                })?;
            }
            regions += 1;
        }

        Ok(AppliedMappingStats { regions, pages })
    }

    pub fn apply_shm_mapping(
        &mut self,
        start: u64,
        end: u64,
        physical_frames: &[usize],
        flags: u32,
        alloc: &mut crate::hal::paging::PageAllocWrapper,
    ) -> Result<(), ApplyMappingError> {
        self.map_range(start, end, flags, alloc, |i, _| {
            physical_frames.get(i).map(|&p| p as u64).ok_or(ApplyMappingError::InvalidRange)
        })?;
        Ok(())
    }

    pub fn remap_range(
        &mut self,
        start: u64,
        end: u64,
        flags: u32,
        _alloc: &mut crate::hal::paging::PageAllocWrapper,
    ) -> Result<(), ApplyMappingError> {
        if (start & (bits::PAGE_SIZE as u64 - 1)) != 0 || (end & (bits::PAGE_SIZE as u64 - 1)) != 0 {
            return Err(ApplyMappingError::MisalignedRange);
        }

        let count = ((end - start) / bits::PAGE_SIZE as u64) as usize;
        for i in 0..count {
            let va = start + (i as u64 * bits::PAGE_SIZE as u64);
            self.update_page_flags(va, flags).map_err(|_| ApplyMappingError::MappingFailed)?;
        }
        Ok(())
    }
}

#[cfg(target_arch = "aarch64")]
impl PageManager {
    /// Internal helper for bulk mapping virtual ranges.
    fn map_range<F>(
        &mut self,
        start: u64,
        end: u64,
        flags: u32,
        alloc: &mut impl crate::hal::paging::FrameAllocator,
        mut get_phys: F,
    ) -> Result<usize, ApplyMappingError>
    where
        F: FnMut(usize, &mut dyn crate::hal::paging::FrameAllocator) -> Result<u64, ApplyMappingError>,
    {
        if (start & (bits::PAGE_SIZE as u64 - 1)) != 0 || (end & (bits::PAGE_SIZE as u64 - 1)) != 0 {
            return Err(ApplyMappingError::MisalignedRange);
        }

        let mut pages = 0usize;
        let count = ((end - start) / bits::PAGE_SIZE as u64) as usize;
        for i in 0..count {
            let va = start + (i as u64 * bits::PAGE_SIZE as u64);
            let pa = get_phys(i, alloc)?;
            self.map_page(va, pa, flags, alloc).map_err(|_| ApplyMappingError::MappingFailed)?;
            pages += 1;
        }
        Ok(pages)
    }

    pub fn apply_virtual_mapping_plan(
        &mut self,
        mappings: &[crate::kernel::module_loader::VirtualMappingPlan],
        flags: u32,
        alloc: &mut impl crate::hal::paging::FrameAllocator,
    ) -> Result<AppliedMappingStats, ApplyMappingError> {
        let mut regions = 0usize;
        let mut pages = 0usize;

        for mapping in mappings {
            pages += self.map_range(mapping.start, mapping.end, flags, alloc, |_, a: &mut dyn crate::hal::paging::FrameAllocator| {
                a.allocate_frame().ok_or(ApplyMappingError::OutOfPhysicalMemory)
            })?;
            regions += 1;
        }

        Ok(AppliedMappingStats { regions, pages })
    }

    pub fn apply_shm_mapping(
        &mut self,
        start: u64,
        end: u64,
        physical_frames: &[usize],
        flags: u32,
        alloc: &mut impl crate::hal::paging::FrameAllocator,
    ) -> Result<(), ApplyMappingError> {
        self.map_range(start, end, flags, alloc, |i, _| {
            physical_frames.get(i).map(|&p| p as u64).ok_or(ApplyMappingError::InvalidRange)
        })?;
        Ok(())
    }
}

#[cfg(target_arch = "x86_64")]
pub fn active_level_4_table(hhdm_offset: u64) -> &'static mut crate::hal::paging::PageTable {
    

    #[cfg(target_os = "none")]
    {
        let (level_4_table_frame, _) = Cr3::read();
        let phys = level_4_table_frame.start_address();
        let virt = x86_64::VirtAddr::new(phys.as_u64() + hhdm_offset);
        let page_table_ptr: *mut crate::hal::paging::PageTable = virt.as_mut_ptr();
        unsafe { &mut *page_table_ptr }
    }
    #[cfg(not(target_os = "none"))]
    {
        let _ = hhdm_offset;
        // Return a dummy page table for host tests
        static mut DUMMY_TABLE: crate::hal::paging::PageTable = crate::hal::paging::PageTable::new();
        #[allow(static_mut_refs)]
        unsafe { &mut DUMMY_TABLE }
    }
}

#[cfg(target_arch = "aarch64")]
pub fn active_level_4_table(hhdm_offset: u64) -> &'static mut crate::hal::paging::PageTable {
    #[cfg(target_os = "none")]
    {
        let ttbr1: u64;
        unsafe {
            core::arch::asm!("mrs {}, ttbr1_el1", out(reg) ttbr1, options(nomem, nostack));
        }
        // TTBR1_EL1 layout: bits 1..47 contain the physical table base
        let phys = ttbr1 & !0xFFFF000000000001; // mask out ASID and CnP
        let virt = (phys + hhdm_offset) as *mut crate::hal::paging::PageTable;
        unsafe { &mut *virt }
    }
    #[cfg(not(target_os = "none"))]
    {
        let _ = hhdm_offset;
        // Return a dummy page table for host tests
        static mut DUMMY_TABLE: crate::hal::paging::PageTable = crate::hal::paging::PageTable::new();
        #[allow(static_mut_refs)]
        unsafe { &mut DUMMY_TABLE }
    }
}
