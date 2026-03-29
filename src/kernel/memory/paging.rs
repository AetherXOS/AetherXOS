#[cfg(target_arch = "x86_64")]
use super::paging_support::{
    validate_page_aligned_range, validate_shm_mapping_inputs, PAGE_SIZE_BYTES_U64,
};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
};
#[cfg(target_arch = "x86_64")]
use x86_64::VirtAddr;

/// Custom bit in PTE used to mark Copy-on-Write pages.
/// We use bit 9 (one of the OS-available bits in x86_64 page table entries).
#[cfg(target_arch = "x86_64")]
const COW_BIT: u64 = 1 << 9;

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

/// Virtual Memory Manager (VMM).
/// Uses offset mapping (all physical memory mapped at a fixed offset).
#[cfg(target_arch = "x86_64")]
pub struct PageManager {
    mapper: OffsetPageTable<'static>,
    physical_memory_offset: VirtAddr,
}

#[cfg(target_arch = "x86_64")]
impl PageManager {
    /// Initialize VMM with a physical memory offset.
    /// Safety: The caller must guarantee that the complete physical memory is mapped to virtual memory at the passed `physical_memory_offset`.
    pub unsafe fn new(
        physical_memory_offset: VirtAddr,
        level_4_table: &'static mut PageTable,
    ) -> Self {
        // Safety: caller guarantees the passed level-4 table matches the active address-space mapping.
        let mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
        Self {
            mapper,
            physical_memory_offset,
        }
    }

    #[inline(always)]
    pub fn physical_offset(&self) -> VirtAddr {
        self.physical_memory_offset
    }

    /// Demand Paging / Copy-on-Write: Allocates a physical frame for a faulting virtual address.
    /// If the page is already mapped with COW semantics (read-only + COW bit), performs a copy.
    pub fn handle_page_fault(
        &mut self,
        addr: VirtAddr,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<(), &'static str> {
        let page = Page::<Size4KiB>::containing_address(addr);

        use x86_64::structures::paging::Translate;
        if self.mapper.translate_addr(addr).is_some() {
            // Page is already mapped — check if this is a CoW fault.
            // A CoW page is mapped as read-only with the COW bit set.
            use x86_64::structures::paging::mapper::TranslateResult;
            match self.mapper.translate(addr) {
                TranslateResult::Mapped {
                    frame,
                    offset: _,
                    flags,
                } => {
                    let raw_flags = flags.bits();
                    let is_cow =
                        (raw_flags & COW_BIT) != 0 && !flags.contains(PageTableFlags::WRITABLE);
                    if !is_cow {
                        return Err("Page already mapped (not CoW)");
                    }
                    // CoW: allocate new frame, copy data, remap as writable
                    let new_frame = frame_allocator
                        .allocate_frame()
                        .ok_or("Out of physical memory for CoW")?;

                    // Copy the old frame content to the new frame
                    let old_phys = frame.start_address().as_u64();
                    let new_phys = new_frame.start_address().as_u64();
                    let old_virt = self.physical_memory_offset + old_phys;
                    let new_virt = self.physical_memory_offset + new_phys;
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            old_virt.as_ptr::<u8>(),
                            new_virt.as_mut_ptr::<u8>(),
                            4096,
                        );
                    }

                    // Unmap old, remap to new frame with WRITABLE and without COW bit
                    let new_flags = (flags | PageTableFlags::WRITABLE)
                        & PageTableFlags::from_bits_truncate(!COW_BIT);
                    unsafe {
                        if let Ok((_, tlb)) = self.mapper.unmap(page) {
                            tlb.flush();
                        }
                        match self
                            .mapper
                            .map_to(page, new_frame, new_flags, frame_allocator)
                        {
                            Ok(tlb) => tlb.flush(),
                            Err(_) => return Err("CoW remap failed"),
                        }
                    }
                    return Ok(());
                }
                _ => return Err("Page already mapped"),
            }
        }

        // 2. Not mapped: allocate a new physical frame (demand paging)
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("Out of physical memory")?;

        // 3. Map the page to the frame
        let flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

        unsafe {
            match self.mapper.map_to(page, frame, flags, frame_allocator) {
                Ok(tlb) => tlb.flush(),
                Err(_) => return Err("Mapping failed"),
            };
        }

        Ok(())
    }

    /// Mark a range of pages as Copy-on-Write (read-only + COW bit).
    /// Used when forking a process to share pages until write occurs.
    pub fn mark_cow_range(&mut self, start: u64, end: u64) -> Result<usize, ApplyMappingError> {
        let page_count = validate_page_aligned_range(start, end)?;
        let mut marked = 0usize;

        let mut addr = start;
        for _ in 0..page_count {
            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(addr));

            // Read current flags, set COW bit, remove WRITABLE
            unsafe {
                if let Ok((old_frame, tlb)) = self.mapper.unmap(page) {
                    tlb.flush();
                    let cow_flags = PageTableFlags::PRESENT
                        | PageTableFlags::USER_ACCESSIBLE
                        | PageTableFlags::from_bits_truncate(COW_BIT);
                    // Use a dummy allocator since we're remapping to the same frame
                    match self.mapper.map_to_with_table_flags(
                        page,
                        old_frame,
                        cow_flags,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE,
                        &mut CowDummyAllocator,
                    ) {
                        Ok(t) => {
                            t.flush();
                            marked += 1;
                        }
                        Err(_) => {}
                    }
                }
            }
            addr += PAGE_SIZE_BYTES_U64;
        }

        Ok(marked)
    }

    /// Map a specific page to a specific frame.
    pub fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<(), &'static str> {
        unsafe {
            match self.mapper.map_to(page, frame, flags, frame_allocator) {
                Ok(tlb) => {
                    tlb.flush();
                    Ok(())
                }
                Err(_) => Err("Mapping failed"),
            }
        }
    }

    pub fn apply_virtual_mapping_plan(
        &mut self,
        mappings: &[crate::kernel::module_loader::VirtualMappingPlan],
        flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<AppliedMappingStats, ApplyMappingError> {
        let mut regions = 0usize;
        let mut pages = 0usize;

        for mapping in mappings {
            let page_count = validate_page_aligned_range(mapping.start, mapping.end)?;

            let mut addr = mapping.start;
            for _ in 0..page_count {
                let page = Page::<Size4KiB>::containing_address(VirtAddr::new(addr));
                let frame = frame_allocator
                    .allocate_frame()
                    .ok_or(ApplyMappingError::OutOfPhysicalMemory)?;

                unsafe {
                    match self.mapper.map_to(page, frame, flags, frame_allocator) {
                        Ok(tlb) => tlb.flush(),
                        Err(_) => return Err(ApplyMappingError::MappingFailed),
                    }
                }

                pages = pages.saturating_add(1);
                addr = addr.saturating_add(PAGE_SIZE_BYTES_U64);
            }

            regions = regions.saturating_add(1);
        }

        Ok(AppliedMappingStats { regions, pages })
    }

    /// Map a shared memory region to a virtual range.
    pub fn apply_shm_mapping(
        &mut self,
        start: u64,
        end: u64,
        physical_frames: &[usize],
        flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<(), ApplyMappingError> {
        let page_count = validate_shm_mapping_inputs(start, end, physical_frames.len())?;

        let mut addr = start;
        for i in 0..page_count {
            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(addr));
            let frame =
                PhysFrame::containing_address(x86_64::PhysAddr::new(physical_frames[i] as u64));

            unsafe {
                match self.mapper.map_to(page, frame, flags, frame_allocator) {
                    Ok(tlb) => tlb.flush(),
                    Err(_) => return Err(ApplyMappingError::MappingFailed),
                }
            }
            addr += 4096;
        }
        Ok(())
    }

    /// Change page flags for an existing mapped virtual range. This will unmap each page
    /// and remap it to the same physical frame with `new_flags`. The operation reuses
    /// the existing physical frames and flushes the TLB for each page.
    pub fn remap_range(
        &mut self,
        start: u64,
        end: u64,
        new_flags: PageTableFlags,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<(), ApplyMappingError> {
        let page_count = validate_page_aligned_range(start, end)?;

        let mut addr = start;
        for _ in 0..page_count {
            let va = VirtAddr::new(addr);
            let page = Page::<Size4KiB>::containing_address(va);

            // Unmap the page and remap it to the same physical frame with new flags.
            unsafe {
                match self.mapper.unmap(page) {
                    Ok((old_frame, tlb)) => {
                        tlb.flush();
                        match self
                            .mapper
                            .map_to(page, old_frame, new_flags, frame_allocator)
                        {
                            Ok(t) => t.flush(),
                            Err(_) => return Err(ApplyMappingError::MappingFailed),
                        }
                    }
                    Err(_) => return Err(ApplyMappingError::MappingFailed),
                }
            }

            addr = addr.saturating_add(PAGE_SIZE_BYTES_U64);
        }

        Ok(())
    }
}

/// Dummy frame allocator used only for CoW remapping where no new frames are needed.
#[cfg(target_arch = "x86_64")]
struct CowDummyAllocator;

#[cfg(target_arch = "x86_64")]
unsafe impl FrameAllocator<Size4KiB> for CowDummyAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        None
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use crate::interfaces::cpu::CpuRegisters;

    let phys = x86_64::PhysAddr::new(crate::hal::cpu::ArchCpuRegisters::read_page_table_root());
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    // Safety: caller guarantees the physical offset maps the current active level-4 table uniquely.
    unsafe { &mut *page_table_ptr }
}

// ═══════════════════════════════════════════════════════════════════════════════
// AArch64 Page Table Management
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "aarch64")]
pub mod aarch64_mmu;
