#![cfg(feature = "paging_enable")]

use x86_64::structures::idt::PageFaultErrorCode;

/// Main entry point for handling user-mode page faults.
/// 
/// This function attempts to resolve a page fault by:
/// 1. Checking if the address is within a valid heap range.
/// 2. Checking if the address is within a registered mmap region.
/// 3. Performing Copy-on-Write (COW) if necessary.
/// 4. Allocating a physical page on-demand for anonymous mappings.
pub fn handle_user_page_fault(addr: u64, error_code: PageFaultErrorCode) -> Result<(), &'static str> {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let tid = crate::interfaces::task::TaskId(cpu.current_task.load(core::sync::atomic::Ordering::Relaxed));
    
    if tid.0 == 0 {
        return Err("Kernel-mode page fault (no current task)");
    }

    let task = crate::kernel::task::get_task(tid).ok_or("Task not found")?;
    let pid = {
        let t = task.lock();
        t.process_id.ok_or("Task not associated with a process")?
    };

    let process = crate::kernel::launch::process_arc_by_id(pid).ok_or("Process not found")?;
    
    // 1. Check Heap
    let (heap_start, heap_break) = {
        (process.heap_start.load(core::sync::atomic::Ordering::Relaxed),
         process.heap_break.load(core::sync::atomic::Ordering::Relaxed))
    };

    if addr >= heap_start && addr < heap_break {
        return allocate_page_for_process(&process, addr, true, None);
    }

    // 2. Check Mmap Regions
    let mappings = process.mappings.lock();
    for mapping in mappings.iter() {
        if addr >= mapping.start && addr < mapping.end {
            // Found a valid mapping. 
            
            // 3. Handle COW (Copy-on-Write)
            if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) && error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
                // If it's a COW page, the HAL should handle it via pmgr.handle_page_fault
                let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
                let offset = x86_64::VirtAddr::new(hhdm);
                let lvl4 = unsafe { &mut *( (process.cr3.as_u64() + hhdm) as *mut x86_64::structures::paging::PageTable ) };
                let mut pmgr = crate::kernel::memory::paging::PageManager {
                    mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
                    physical_memory_offset: offset,
                };
                let mut frame_allocator = crate::hal::paging::PageAllocWrapper;
                if let Ok(_) = pmgr.handle_page_fault(addr, &mut frame_allocator) {
                    return Ok(());
                }
            }

            let _writable = (mapping.prot & crate::modules::posix_consts::mman::PROT_READ) != 0; // Wait, prot is usually POSIX flags
            // Use POSIX constants for mapping.prot
            use crate::modules::posix_consts::mman as m;
            let is_writable = (mapping.prot & m::PROT_WRITE) != 0;

            // 4. Resolve Fault (Anonymous or File-Backed)
            let page_addr = addr & !4095;
            
            if mapping.map_id >= 2_000_000 {
                // Shared Memory: Fetch existing frame
                return resolve_shm_fault(&process, mapping.map_id, page_addr, mapping.start, is_writable);
            } else if mapping.map_id >= 1_000_000 {
                // Anonymous: Zero-fill new frame
                return allocate_page_for_process(&process, page_addr, is_writable, None);
            } else {
                // File-Backed: Load from VFS
                return resolve_file_fault(&process, mapping.map_id, page_addr, mapping.start, is_writable);
            }
        }
    }

    Err("Segmentation fault: Address not in any valid region")
}

fn resolve_shm_fault(process: &crate::kernel::process::Process, map_id: u32, page_addr: u64, map_start: u64, writable: bool) -> Result<(), &'static str> {
    #[cfg(feature = "ipc_shared_memory")]
    if let Some(shm) = crate::modules::ipc::shared_memory::shm_get_region(map_id as i32) {
        let page_idx = ((page_addr - map_start) / 4096) as usize;
        if page_idx < shm.physical_pages.len() {
            let phys = shm.physical_pages[page_idx] as u64;
            return map_existing_frame(process, page_addr, phys, writable);
        }
    }
    Err("SHM region not found")
}

fn resolve_file_fault(process: &crate::kernel::process::Process, map_id: u32, page_addr: u64, map_start: u64, writable: bool) -> Result<(), &'static str> {
    // 1. Allocate a page
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let _frame = allocate_page_for_process(process, page_addr, writable, None)?;
    
    // 2. Read data from VFS
    let offset = (page_addr - map_start) as usize;
    let _kernel_vaddr = page_addr + hhdm; // Wait, page_addr is user vaddr, we need to find the KERNEL vaddr of the newly allocated frame
    // Actually allocate_page_for_process already mapped it.
    // We can use the HHDM of the physical frame.
    
    // Let's re-find the frame we just mapped
    let phys = translate_user_vaddr(process.cr3.as_u64(), page_addr)?;
    let dest_ptr = (phys + hhdm) as *mut u8;
    let dest_slice = unsafe { core::slice::from_raw_parts_mut(dest_ptr, 4096) };
    
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    if let Err(_) = crate::modules::posix::mman::mmap_read(map_id, dest_slice, offset) {
        return Err("VFS read failed during page fault");
    }
    
    #[cfg(not(all(feature = "vfs", feature = "posix_mman")))]
    return Err("Mmap not supported without VFS and POSIX mman features");
    
    Ok(())
}

fn map_existing_frame(process: &crate::kernel::process::Process, vaddr: u64, phys: u64, writable: bool) -> Result<(), &'static str> {
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let offset = x86_64::VirtAddr::new(hhdm);
    let lvl4 = unsafe { &mut *( (process.cr3.as_u64() + hhdm) as *mut x86_64::structures::paging::PageTable ) };
    let mut page_manager = crate::kernel::memory::paging::PageManager {
        mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
        physical_memory_offset: offset,
    };
    let mut frame_allocator = crate::hal::paging::PageAllocWrapper;
    
    use crate::interfaces::memory::page_flags as bits;
    let mut flags = bits::PRESENT | bits::USER;
    if writable { flags |= bits::WRITABLE; }

    page_manager.map_page(vaddr, phys, flags, &mut frame_allocator)
}

fn allocate_page_for_process(process: &crate::kernel::process::Process, vaddr: u64, writable: bool, source_phys: Option<u64>) -> Result<(), &'static str> {
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let offset = x86_64::VirtAddr::new(hhdm);
    let lvl4 = unsafe { &mut *( (process.cr3.as_u64() + hhdm) as *mut x86_64::structures::paging::PageTable ) };
    
    let mut page_manager = crate::kernel::memory::paging::PageManager {
        mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
        physical_memory_offset: offset,
    };
    
    let mut frame_allocator = crate::hal::paging::PageAllocWrapper;
    
    use crate::interfaces::memory::page_flags as bits;
    let mut flags = bits::PRESENT | bits::USER;
    if writable {
        flags |= bits::WRITABLE;
    }

    let page_addr = vaddr & !4095;
    let frame = frame_allocator.allocate_frame().ok_or("OOM during demand paging")?;
    let phys = frame.start_address().as_u64();

    // Initialize the page
    unsafe {
        let ptr = (phys + hhdm) as *mut u8;
        if let Some(src) = source_phys {
            let src_ptr = (src + hhdm) as *const u8;
            core::ptr::copy_nonoverlapping(src_ptr, ptr, 4096);
        } else {
            core::ptr::write_bytes(ptr, 0, 4096);
        }
    }

    match page_manager.map_page(page_addr, phys, flags, &mut frame_allocator) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn translate_user_vaddr(cr3: u64, vaddr: u64) -> Result<u64, &'static str> {
    use x86_64::structures::paging::Translate;
    use x86_64::VirtAddr;
    
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let l4_table_ptr = (cr3 + hhdm) as *mut x86_64::structures::paging::PageTable;
    let l4_table = unsafe { &mut *l4_table_ptr };
    
    let mapper = unsafe { x86_64::structures::paging::OffsetPageTable::new(l4_table, VirtAddr::new(hhdm)) };
    match mapper.translate_addr(VirtAddr::new(vaddr)) {
        Some(phys) => Ok(phys.as_u64()),
        None => Err("Address not mapped"),
    }
}
