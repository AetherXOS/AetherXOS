use super::*;

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn install_lazy_plt_trampolines(
    process: &crate::kernel::process::Process,
    runtime_hooks: &crate::kernel::process::RuntimeLifecycleHooks,
    prepared: &mut PreparedProcessImage,
    loader: &mut crate::kernel::dynamic_linker::so_loader::SharedObjectLoader,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
) {
    // Install lazy PLT trampolines: allocate an executable trampoline region
    // and write a per-slot trampoline that performs the `RESOLVE_PLT` syscall.
    if !loader.plt_slots.is_empty() {
        const TRAMP_SIZE: u64 = 0x20;
        let count = loader.plt_slots.len() as u64;
        if let Some(total) = TRAMP_SIZE.checked_mul(count) {
            // Find highest mapped end
            let max_end = prepared
                .mappings
                .iter()
                .map(|m| m.end)
                .max()
                .unwrap_or(prepared.load_plan.entry);
            let tramp_start = align_up(max_end, PAGE_SIZE).unwrap_or(max_end + PAGE_SIZE);
            let tramp_end = align_up(tramp_start + total, PAGE_SIZE).unwrap_or(tramp_start + total);

            // Allocate trampoline pages as writable but non-executable (W, !X).
            // We'll write trampolines and then remap the range to RX (remove write).
            let prot_write =
                crate::modules::posix_consts::mman::PROT_READ | crate::modules::posix_consts::mman::PROT_WRITE;
            if materialize_virtual_mapping_range(
                tramp_start,
                tramp_end,
                prot_write as u32,
                page_manager,
                frame_allocator,
            )
            .is_ok()
            {
                // Write trampolines and patch GOT slots to point to them.
                for (i, (slot_vaddr, _name, is_ifunc, resolver_vaddr)) in
                    loader.plt_slots.iter().enumerate()
                {
                    let tramp_addr = tramp_start + (i as u64) * TRAMP_SIZE;
                    let mut buf = [0u8; TRAMP_SIZE as usize];
                    let mut off = 0usize;
                    if *is_ifunc {
                        // IFUNC: call resolver function in user-space, store result, jump
                        // mov rax, resolver_vaddr
                        buf[off] = 0x48;
                        buf[off + 1] = 0xB8;
                        off += 2;
                        buf[off..off + 8].copy_from_slice(&resolver_vaddr.to_le_bytes());
                        off += 8;
                        // call rax
                        buf[off] = 0xFF;
                        buf[off + 1] = 0xD0;
                        off += 2;
                        // mov rdi, slot_vaddr
                        buf[off] = 0x48;
                        buf[off + 1] = 0xBF;
                        off += 2;
                        buf[off..off + 8].copy_from_slice(&slot_vaddr.to_le_bytes());
                        off += 8;
                        // mov [rdi], rax
                        buf[off] = 0x48;
                        buf[off + 1] = 0x89;
                        buf[off + 2] = 0x07;
                        off += 3;
                        // jmp rax
                        buf[off] = 0xFF;
                        buf[off + 1] = 0xE0;
                        off += 2;
                    } else {
                        // Non-IFUNC: perform syscall resolver
                        // mov rax, RESOLVE_PLT
                        buf[off] = 0x48;
                        buf[off + 1] = 0xB8;
                        off += 2;
                        buf[off..off + 8].copy_from_slice(
                            &(crate::kernel::syscalls::syscalls_consts::RESOLVE_PLT as u64)
                                .to_le_bytes(),
                        );
                        off += 8;
                        // mov rdi, slot_vaddr
                        buf[off] = 0x48;
                        buf[off + 1] = 0xBF;
                        off += 2;
                        buf[off..off + 8].copy_from_slice(&slot_vaddr.to_le_bytes());
                        off += 8;
                        // xor rsi, rsi
                        buf[off] = 0x48;
                        buf[off + 1] = 0x31;
                        buf[off + 2] = 0xF6;
                        off += 3;
                        // syscall
                        buf[off] = 0x0F;
                        buf[off + 1] = 0x05;
                        off += 2;
                        // jmp rax
                        buf[off] = 0xFF;
                        buf[off + 1] = 0xE0;
                        off += 2;
                    }

                    // Write trampoline bytes into process memory
                    let _ = materialize_write_bytes_at(tramp_addr, &buf[..off], &prepared.mappings);
                    // Patch GOT/JMP_SLOT entry to point to trampoline
                    let _ = materialize_write_u64_at(*slot_vaddr, tramp_addr, &prepared.mappings);
                }

                // Now remap trampoline pages to read+exec (remove write) to enforce W^X.
                use x86_64::structures::paging::PageTableFlags;
                let target_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
                match page_manager.remap_range(
                    tramp_start,
                    tramp_end,
                    target_flags,
                    frame_allocator,
                ) {
                    Ok(()) => {}
                    Err(e) => {
                        klog_warn!(
                            "trampoline remap failed for range {:#x}-{:#x}: {:?}",
                            tramp_start,
                            tramp_end,
                            e
                        );
                    }
                }
            }
        }

        if let Some(runtime_entry) = install_runtime_init_trampoline(
            process,
            runtime_hooks,
            prepared.load_plan.entry,
            &mut prepared.mappings,
            page_manager,
            frame_allocator,
            &super::NEXT_RUNTIME_TRAMPOLINE_MAP_ID,
            crate::interfaces::memory::PAGE_SIZE_4K as u64,
        ) {
            if runtime_entry != 0 {
                process.set_runtime_entry(Some(runtime_entry));
                prepared.load_plan.entry = runtime_entry;
            }
        }
        if let Some(runtime_fini_entry) = install_runtime_fini_trampoline(
            process,
            runtime_hooks,
            &mut prepared.mappings,
            page_manager,
            frame_allocator,
            &super::NEXT_RUNTIME_TRAMPOLINE_MAP_ID,
            crate::interfaces::memory::PAGE_SIZE_4K as u64,
        ) {
            if runtime_fini_entry != 0 {
                process.set_runtime_fini_entry(Some(runtime_fini_entry));
            }
        }
    }
}
