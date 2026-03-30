pub use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags as X86Flags, PhysFrame, Size4KiB,
};
pub use x86_64::structures::paging::FrameAllocator as X86FrameAllocator;
use x86_64::VirtAddr;
use crate::interfaces::memory::page_flags as bits;
use x86_64::structures::paging::Translate;

pub const COW_BIT: u64 = 1 << 9; // OS-available bit

pub struct PageManager {
    pub mapper: OffsetPageTable<'static>,
    pub physical_memory_offset: VirtAddr,
}

fn convert_flags(flags: u32) -> X86Flags {
    let mut f = X86Flags::empty();
    if (flags & bits::PRESENT) != 0 { f |= X86Flags::PRESENT; }
    if (flags & bits::WRITABLE) != 0 { f |= X86Flags::WRITABLE; }
    if (flags & bits::USER) != 0 { f |= X86Flags::USER_ACCESSIBLE; }
    if (flags & bits::NO_EXECUTE) != 0 { f |= X86Flags::NO_EXECUTE; }
    if (flags & bits::COW) != 0 { f |= X86Flags::from_bits_truncate(COW_BIT); }
    f
}

impl PageManager {
    pub unsafe fn new(physical_memory_offset: VirtAddr, level_4_table: &'static mut PageTable) -> Self {
        let mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
        Self { mapper, physical_memory_offset }
    }

    pub fn map_page(
        &mut self,
        va: u64,
        pa: u64,
        flags: u32,
        alloc: &mut impl X86FrameAllocator<Size4KiB>,
    ) -> Result<(), &'static str> {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(va));
        let frame = PhysFrame::containing_address(x86_64::PhysAddr::new(pa));
        let x86_flags = convert_flags(flags);
        unsafe {
            match self.mapper.map_to(page, frame, x86_flags, alloc) {
                Ok(tlb) => { tlb.flush(); Ok(()) }
                Err(_) => Err("Mapping failed"),
            }
        }
    }

    pub fn handle_page_fault(
        &mut self,
        va: u64,
        alloc: &mut impl X86FrameAllocator<Size4KiB>,
    ) -> Result<(), &'static str> {
        let addr = VirtAddr::new(va);
        let page = Page::<Size4KiB>::containing_address(addr);

        match self.mapper.translate(addr) {
            x86_64::structures::paging::mapper::TranslateResult::Mapped { frame, flags, .. } => {
                if (flags.bits() & COW_BIT) != 0 {
                    let new_frame = alloc.allocate_frame().ok_or("Out of memory")?;
                    let old_virt = self.physical_memory_offset + frame.start_address().as_u64();
                    let new_virt = self.physical_memory_offset + new_frame.start_address().as_u64();
                    unsafe {
                        core::ptr::copy_nonoverlapping(old_virt.as_ptr::<u8>(), new_virt.as_mut_ptr::<u8>(), 4096);
                        let new_flags = (flags | X86Flags::WRITABLE) & X86Flags::from_bits_truncate(!COW_BIT);
                        self.mapper.unmap(page).unwrap().1.flush();
                        self.mapper.map_to(page, new_frame, new_flags, alloc).unwrap().flush();
                    }
                    return Ok(());
                }
                Err("Already mapped")
            }
            _ => {
                let frame = alloc.allocate_frame().ok_or("Out of memory")?;
                let flags = X86Flags::PRESENT | X86Flags::WRITABLE | X86Flags::USER_ACCESSIBLE;
                unsafe { self.mapper.map_to(page, frame, flags, alloc).unwrap().flush(); }
                Ok(())
            }
        }
    }
}
