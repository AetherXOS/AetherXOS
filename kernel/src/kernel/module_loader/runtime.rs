#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn materialize_write_bytes_at(
    vaddr: u64,
    data: &[u8],
    mappings: &[super::VirtualMappingPlan],
) -> Result<(), ()> {
    #[cfg(target_os = "none")]
    for m in mappings {
        if vaddr >= m.start && vaddr + (data.len() as u64) <= m.end {
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), vaddr as *mut u8, data.len());
            }
            return Ok(());
        }
    }
    #[cfg(not(target_os = "none"))]
    {
        let _ = (vaddr, data, mappings);
        return Ok(());
    }
    #[cfg(target_os = "none")]
    Err(())
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn materialize_write_u64_at(
    vaddr: u64,
    value: u64,
    mappings: &[super::VirtualMappingPlan],
) -> Result<(), ()> {
    #[cfg(target_os = "none")]
    for m in mappings {
        if vaddr >= m.start && vaddr + 8 <= m.end {
            unsafe {
                core::ptr::write_unaligned(vaddr as *mut u64, value);
            }
            return Ok(());
        }
    }
    #[cfg(not(target_os = "none"))]
    {
        let _ = (vaddr, value, mappings);
        return Ok(());
    }
    #[cfg(target_os = "none")]
    Err(())
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn install_runtime_init_trampoline(
    process: &crate::kernel::process::Process,
    hooks: &crate::kernel::process::RuntimeLifecycleHooks,
    original_entry: u64,
    mappings: &mut alloc::vec::Vec<super::VirtualMappingPlan>,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut crate::hal::paging::PageAllocWrapper,
    next_runtime_trampoline_map_id: &core::sync::atomic::AtomicU64,
    page_size: u64,
) -> Option<u64> {
    let ordered_hooks = hooks.ordered_init_calls();
    if ordered_hooks.is_empty() {
        return None;
    }

    let max_end = mappings
        .iter()
        .map(|m| m.end)
        .max()
        .unwrap_or(original_entry);
    let tramp_start = super::module_loader_support::align_up(max_end, page_size)?;
    let tramp_end =
        super::module_loader_support::align_up(tramp_start + page_size, page_size)?;
    let prot_write = crate::modules::posix_consts::mman::PROT_READ
        | crate::modules::posix_consts::mman::PROT_WRITE;
    super::materialize_virtual_mapping_range(
        tramp_start,
        tramp_end,
        prot_write as u32,
        page_manager,
        frame_allocator,
    )
    .ok()?;

    let mut trampoline = [0u8; super::module_loader_support::PAGE_SIZE as usize];
    let used = crate::hal::platforms::get_platform().encode_init_trampoline(&mut trampoline, &ordered_hooks, original_entry)?;
    materialize_write_bytes_at(tramp_start, &trampoline[..used], mappings).ok()?;

    use x86_64::structures::paging::PageTableFlags;
    let target_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    let _ = page_manager.remap_range(tramp_start, tramp_end, target_flags.bits() as u32, frame_allocator);

    let map_id = next_runtime_trampoline_map_id
        .fetch_add(1, core::sync::atomic::Ordering::Relaxed) as u32;
    let prot_exec = crate::modules::posix_consts::mman::PROT_READ
        | crate::modules::posix_consts::mman::PROT_EXEC;
    let map_flags = crate::modules::posix_consts::mman::MAP_PRIVATE as u32;
    let _ =
        process.register_mapping(map_id, tramp_start, tramp_end, prot_exec as u32, map_flags);
    mappings.push(super::VirtualMappingPlan {
        start: tramp_start,
        end: tramp_end,
        virtual_addr: tramp_start, mem_size: tramp_end - tramp_start, file_bytes: used as u64,
        zero_fill_bytes: tramp_end.saturating_sub(tramp_start + used as u64), file_offset: 0,
    });
    Some(tramp_start)
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn install_runtime_fini_trampoline(
    process: &crate::kernel::process::Process,
    hooks: &crate::kernel::process::RuntimeLifecycleHooks,
    mappings: &mut alloc::vec::Vec<super::VirtualMappingPlan>,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut crate::hal::paging::PageAllocWrapper,
    next_runtime_trampoline_map_id: &core::sync::atomic::AtomicU64,
    page_size: u64,
) -> Option<u64> {
    let ordered_hooks = hooks.ordered_fini_calls();
    if ordered_hooks.is_empty() {
        return None;
    }

    let max_end = mappings.iter().map(|m| m.end).max().unwrap_or(0);
    let tramp_start = super::module_loader_support::align_up(max_end, page_size)?;
    let tramp_end =
        super::module_loader_support::align_up(tramp_start + page_size, page_size)?;
    let prot_write = crate::modules::posix_consts::mman::PROT_READ
        | crate::modules::posix_consts::mman::PROT_WRITE;
    super::materialize_virtual_mapping_range(
        tramp_start,
        tramp_end,
        prot_write as u32,
        page_manager,
        frame_allocator,
    )
    .ok()?;

    let mut trampoline = [0u8; super::module_loader_support::PAGE_SIZE as usize];
    let used = crate::hal::platforms::get_platform().encode_fini_trampoline(&mut trampoline, &ordered_hooks)?;
    materialize_write_bytes_at(tramp_start, &trampoline[..used], mappings).ok()?;

    use x86_64::structures::paging::PageTableFlags;
    let target_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    let _ = page_manager.remap_range(tramp_start, tramp_end, target_flags.bits() as u32, frame_allocator);

    let map_id = next_runtime_trampoline_map_id
        .fetch_add(1, core::sync::atomic::Ordering::Relaxed) as u32;
    let prot_exec = crate::modules::posix_consts::mman::PROT_READ
        | crate::modules::posix_consts::mman::PROT_EXEC;
    let map_flags = crate::modules::posix_consts::mman::MAP_PRIVATE as u32;
    let _ =
        process.register_mapping(map_id, tramp_start, tramp_end, prot_exec as u32, map_flags);
    mappings.push(super::VirtualMappingPlan {
        start: tramp_start,
        end: tramp_end,
        virtual_addr: tramp_start, mem_size: tramp_end - tramp_start, file_bytes: used as u64,
        zero_fill_bytes: tramp_end.saturating_sub(tramp_start + used as u64), file_offset: 0,
    });
    Some(tramp_start)
}
