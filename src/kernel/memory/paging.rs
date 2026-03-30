//! Platform-agnostic Virtual Memory Manager (VMM).
//! Delegated to the Hardware Abstraction Layer (HAL) for platform-specifics.

use crate::kernel::bit_utils::paging as bits;
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
        alloc: &mut impl crate::hal::paging::FrameAllocator<crate::hal::paging::Size4KiB>,
        mut get_phys: F,
    ) -> Result<usize, ApplyMappingError>
    where
        F: FnMut(usize, &mut dyn crate::hal::paging::FrameAllocator<crate::hal::paging::Size4KiB>) -> Result<u64, ApplyMappingError>,
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
        alloc: &mut impl crate::hal::paging::FrameAllocator<crate::hal::paging::Size4KiB>,
    ) -> Result<AppliedMappingStats, ApplyMappingError> {
        let mut regions = 0usize;
        let mut pages = 0usize;

        for mapping in mappings {
            pages += self.map_range(mapping.start, mapping.end, flags, alloc, |_, a: &mut dyn crate::hal::paging::FrameAllocator<crate::hal::paging::Size4KiB>| {
                a.allocate_frame().map(|f| f.start_address().as_u64()).ok_or(ApplyMappingError::OutOfPhysicalMemory)
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
        alloc: &mut impl crate::hal::paging::FrameAllocator<crate::hal::paging::Size4KiB>,
    ) -> Result<(), ApplyMappingError> {
        self.map_range(start, end, flags, alloc, |i, _| {
            physical_frames.get(i).map(|&p| p as u64).ok_or(ApplyMappingError::InvalidRange)
        })?;
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
