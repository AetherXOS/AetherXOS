#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn materialize_write_bytes_at(
    vaddr: u64,
    data: &[u8],
    mappings: &[crate::kernel::process::MappingRecord],
) -> Result<(), ()> {
    for m in mappings {
        if vaddr >= m.start && vaddr + (data.len() as u64) <= m.end {
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), vaddr as *mut u8, data.len());
            }
            return Ok(());
        }
    }
    Err(())
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn materialize_write_u64_at(
    vaddr: u64,
    value: u64,
    mappings: &[crate::kernel::process::MappingRecord],
) -> Result<(), ()> {
    for m in mappings {
        if vaddr >= m.start && vaddr + 8 <= m.end {
            unsafe {
                core::ptr::write_unaligned(vaddr as *mut u64, value);
            }
            return Ok(());
        }
    }
    Err(())
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn install_runtime_init_trampoline(
    process: &crate::kernel::process::Process,
    hooks: &crate::kernel::process::RuntimeLifecycleHooks,
    original_entry: u64,
    mappings: &mut alloc::vec::Vec<super::VirtualMappingPlan>,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
    next_runtime_trampoline_map_id: &core::sync::atomic::AtomicU64,
    page_size: u64,
) -> Option<u64> {
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (
            process,
            hooks,
            original_entry,
            mappings,
            page_manager,
            frame_allocator,
            next_runtime_trampoline_map_id,
            page_size,
        );
        None
    }

    #[cfg(target_arch = "x86_64")]
    {
        let ordered_hooks = hooks.ordered_init_calls();
        if ordered_hooks.is_empty() {
            return None;
        }

        let max_end = mappings
            .iter()
            .map(|m| m.end)
            .max()
            .unwrap_or(original_entry);
        let tramp_start = super::module_loader_support::align_up(max_end, page_size).ok()?;
        let tramp_end =
            super::module_loader_support::align_up(tramp_start + page_size, page_size).ok()?;
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
        let used =
            encode_x86_64_runtime_init_trampoline(&mut trampoline, &ordered_hooks, original_entry)?;
        materialize_write_bytes_at(tramp_start, &trampoline[..used], mappings).ok()?;

        use x86_64::structures::paging::PageTableFlags;
        let target_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        let _ = page_manager.remap_range(tramp_start, tramp_end, target_flags, frame_allocator);

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
            file_bytes: used as u64,
            zero_fill_bytes: tramp_end.saturating_sub(tramp_start + used as u64),
        });
        Some(tramp_start)
    }
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn install_runtime_fini_trampoline(
    process: &crate::kernel::process::Process,
    hooks: &crate::kernel::process::RuntimeLifecycleHooks,
    mappings: &mut alloc::vec::Vec<super::VirtualMappingPlan>,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
    next_runtime_trampoline_map_id: &core::sync::atomic::AtomicU64,
    page_size: u64,
) -> Option<u64> {
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (
            process,
            hooks,
            mappings,
            page_manager,
            frame_allocator,
            next_runtime_trampoline_map_id,
            page_size,
        );
        None
    }

    #[cfg(target_arch = "x86_64")]
    {
        let ordered_hooks = hooks.ordered_fini_calls();
        if ordered_hooks.is_empty() {
            return None;
        }

        let max_end = mappings.iter().map(|m| m.end).max().unwrap_or(0);
        let tramp_start = super::module_loader_support::align_up(max_end, page_size).ok()?;
        let tramp_end =
            super::module_loader_support::align_up(tramp_start + page_size, page_size).ok()?;
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
        let used = encode_x86_64_runtime_fini_trampoline(&mut trampoline, &ordered_hooks)?;
        materialize_write_bytes_at(tramp_start, &trampoline[..used], mappings).ok()?;

        use x86_64::structures::paging::PageTableFlags;
        let target_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        let _ = page_manager.remap_range(tramp_start, tramp_end, target_flags, frame_allocator);

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
            file_bytes: used as u64,
            zero_fill_bytes: tramp_end.saturating_sub(tramp_start + used as u64),
        });
        Some(tramp_start)
    }
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn encode_x86_64_runtime_init_trampoline(
    buf: &mut [u8],
    hooks: &[u64],
    final_entry: u64,
) -> Option<usize> {
    fn push_bytes(buf: &mut [u8], off: &mut usize, bytes: &[u8]) -> Option<()> {
        let end = off.checked_add(bytes.len())?;
        if end > buf.len() {
            return None;
        }
        buf[*off..end].copy_from_slice(bytes);
        *off = end;
        Some(())
    }

    fn push_mov_rax_imm64(buf: &mut [u8], off: &mut usize, imm: u64) -> Option<()> {
        push_bytes(buf, off, &[0x48, 0xB8])?;
        push_bytes(buf, off, &imm.to_le_bytes())
    }

    let mut off = 0usize;
    for hook in hooks {
        push_mov_rax_imm64(buf, &mut off, *hook)?;
        push_bytes(buf, &mut off, &[0x48, 0x85, 0xC0])?;
        push_bytes(buf, &mut off, &[0x74, 0x02])?;
        push_bytes(buf, &mut off, &[0xFF, 0xD0])?;
    }
    push_mov_rax_imm64(buf, &mut off, final_entry)?;
    push_bytes(buf, &mut off, &[0xFF, 0xE0])?;
    Some(off)
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn encode_x86_64_runtime_fini_trampoline(
    buf: &mut [u8],
    hooks: &[u64],
) -> Option<usize> {
    fn push_bytes(buf: &mut [u8], off: &mut usize, bytes: &[u8]) -> Option<()> {
        let end = off.checked_add(bytes.len())?;
        if end > buf.len() {
            return None;
        }
        buf[*off..end].copy_from_slice(bytes);
        *off = end;
        Some(())
    }

    fn push_mov_rax_imm64(buf: &mut [u8], off: &mut usize, imm: u64) -> Option<()> {
        push_bytes(buf, off, &[0x48, 0xB8])?;
        push_bytes(buf, off, &imm.to_le_bytes())
    }

    let mut off = 0usize;
    for hook in hooks {
        push_mov_rax_imm64(buf, &mut off, *hook)?;
        push_bytes(buf, &mut off, &[0x48, 0x85, 0xC0])?;
        push_bytes(buf, &mut off, &[0x74, 0x02])?;
        push_bytes(buf, &mut off, &[0xFF, 0xD0])?;
    }
    push_bytes(buf, &mut off, &[0x31, 0xC0])?; // xor eax,eax
    push_bytes(buf, &mut off, &[0xC3])?; // ret
    Some(off)
}
