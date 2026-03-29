use super::*;

mod trampoline;

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub(super) fn apply_dynamic_linking_and_runtime_trampolines(
    process: &crate::kernel::process::Process,
    image: &[u8],
    prepared: &mut PreparedProcessImage,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
) {
    // After segments are materialized into the process address space, apply relocations in-place.
    // Parse ELF dynamic section from the original image to find relocation tables and symbol table.
    if let Ok(elf) = parse_elf(image) {
        // find PT_DYNAMIC
        let mut dyn_off = None;
        let mut dyn_size = None;
        for ph in elf.program_iter() {
            if let Ok(Type::Dynamic) = ph.get_type() {
                dyn_off = Some(ph.offset() as usize);
                dyn_size = Some(ph.file_size() as usize);
                break;
            }
        }
        if let (Some(dyn_off), Some(dyn_size)) = (dyn_off, dyn_size) {
            let dyn_count = dyn_size / 16;
            if let Some(dynamic) = crate::kernel::dynamic_linker::elf_dynamic::DynamicSection::parse(
                image,
                dyn_off as u64,
                dyn_count,
            ) {
                // helper: vaddr -> file offset in original image
                let vaddr_to_off = |vaddr: u64| -> Option<usize> {
                    for ph in elf.program_iter() {
                        if let Ok(t) = ph.get_type() {
                            if matches!(t, Type::Load) {
                                let va = ph.virtual_addr();
                                let memsz = ph.mem_size();
                                if vaddr >= va && vaddr < va + memsz {
                                    let off = ph.offset() + (vaddr - va);
                                    return usize::try_from(off).ok();
                                }
                            }
                        }
                    }
                    None
                };

                // collect DT_NEEDED, load via SharedObjectLoader for symbol resolution
                let mut needed = alloc::vec::Vec::new();
                if let Some(strtab_off) = dynamic.dt_strtab.and_then(|v| vaddr_to_off(v)) {
                    let read_dynstr = |name_off: u64| -> Option<alloc::string::String> {
                        let idx = strtab_off.checked_add(name_off as usize)?;
                        let mut end = idx;
                        while end < image.len() && image[end] != 0 {
                            end += 1;
                        }
                        if end <= idx {
                            return None;
                        }
                        core::str::from_utf8(&image[idx..end])
                            .ok()
                            .map(|s| s.to_string())
                    };
                    for name_off in dynamic.dt_needed.iter() {
                        if let Some(name) = read_dynstr(*name_off) {
                            needed.push(name);
                        }
                    }
                    let split_paths = |raw: &str| -> alloc::vec::Vec<alloc::string::String> {
                        raw.split(':')
                            .map(str::trim)
                            .filter(|part| !part.is_empty())
                            .map(|part| part.to_string())
                            .collect()
                    };
                    let mut search_paths = alloc::vec::Vec::new();
                    if let Some(runpath_off) = dynamic.dt_runpath {
                        if let Some(runpath) = read_dynstr(runpath_off) {
                            search_paths.extend(split_paths(&runpath));
                        }
                    } else if let Some(rpath_off) = dynamic.dt_rpath {
                        if let Some(rpath) = read_dynstr(rpath_off) {
                            search_paths.extend(split_paths(&rpath));
                        }
                    }
                    let mut loader = crate::kernel::dynamic_linker::so_loader::SharedObjectLoader::with_search_paths(&search_paths);
                    loader.load_needed(&needed);

                    // find symtab/strtab and symbol count
                    let symtab_file_off = dynamic.dt_symtab.and_then(|v| vaddr_to_off(v));
                    let strtab_file_off = dynamic.dt_strtab.and_then(|v| vaddr_to_off(v));
                    let mut sym_count = 0usize;
                    if let Some(hash_vaddr) = dynamic.dt_hash {
                        if let Some(hash_off) = vaddr_to_off(hash_vaddr) {
                            sym_count =
                                crate::kernel::elf_dynamic::parse_sysv_hash_nchain(image, hash_off)
                                    .unwrap_or(0);
                        }
                    }
                    if sym_count == 0 {
                        if let Some(hash_vaddr) = dynamic.dt_gnu_hash {
                            if let Some(hash_off) = vaddr_to_off(hash_vaddr) {
                                sym_count =
                                    crate::kernel::elf_dynamic::parse_gnu_hash_symbol_count(
                                        image, hash_off,
                                    )
                                    .unwrap_or(0);
                            }
                        }
                    }

                    if let (Some(symtab_file_off), Some(strtab_file_off)) =
                        (symtab_file_off, strtab_file_off)
                    {
                        if sym_count == 0 {
                            let sym_entry_size = dynamic.dt_syment.unwrap_or(24) as usize;
                            if sym_entry_size != 0 {
                                let symtab_end = dynamic
                                    .dt_strsz
                                    .and_then(|strsz| strtab_file_off.checked_add(strsz as usize))
                                    .unwrap_or(image.len())
                                    .min(image.len());
                                if symtab_end > symtab_file_off {
                                    sym_count = (symtab_end.saturating_sub(symtab_file_off))
                                        / sym_entry_size;
                                }
                            }
                            if sym_count > 1_000_000 {
                                sym_count = 0;
                            }
                        }
                        if sym_count != 0 {
                            // If DT_VERSYM present, compute file offset for versym
                            let versym_off = dynamic.dt_versym.and_then(|v| vaddr_to_off(v));
                            if let Some(symtab) =
                                crate::kernel::dynamic_linker::symbol::SymbolTable::parse(
                                    image,
                                    strtab_file_off as u64,
                                    symtab_file_off as u64,
                                    sym_count,
                                    versym_off.map(|o| o as u64),
                                    None,
                                    None,
                                )
                            {
                                // compute original image base (lowest PT_LOAD vaddr)
                                let mut image_base = u64::MAX;
                                for ph in elf.program_iter() {
                                    if let Ok(Type::Load) = ph.get_type() {
                                        image_base = image_base.min(ph.virtual_addr());
                                    }
                                }
                                if image_base == u64::MAX {
                                    image_base = 0;
                                }
                                let runtime_base = image_base + prepared.load_plan.aslr_base;
                                let runtime_addr = |vaddr: u64| -> u64 {
                                    vaddr
                                        .checked_sub(image_base)
                                        .map(|delta| delta.wrapping_add(runtime_base))
                                        .unwrap_or(vaddr.wrapping_add(prepared.load_plan.aslr_base))
                                };
                                let read_pointer_array =
                                    |array_vaddr: u64, array_size: u64| -> alloc::vec::Vec<u64> {
                                        let mut out = alloc::vec::Vec::new();
                                        let Some(array_off) = vaddr_to_off(array_vaddr) else {
                                            return out;
                                        };
                                        let Some(entries) = usize::try_from(array_size / 8).ok()
                                        else {
                                            return out;
                                        };
                                        for idx in 0..entries {
                                            let off = array_off + idx * 8;
                                            let Some(bytes) = image.get(off..off + 8) else {
                                                break;
                                            };
                                            let raw = u64::from_le_bytes(bytes.try_into().unwrap());
                                            if raw != 0 {
                                                out.push(runtime_addr(raw));
                                            }
                                        }
                                        out
                                    };
                                let runtime_hooks = crate::kernel::process::RuntimeLifecycleHooks {
                                    preinit_array: match (
                                        dynamic.dt_preinit_array,
                                        dynamic.dt_preinit_arraysz,
                                    ) {
                                        (Some(vaddr), Some(size)) => {
                                            read_pointer_array(vaddr, size)
                                        }
                                        _ => alloc::vec::Vec::new(),
                                    },
                                    init: dynamic.dt_init.map(runtime_addr),
                                    init_array: match (
                                        dynamic.dt_init_array,
                                        dynamic.dt_init_arraysz,
                                    ) {
                                        (Some(vaddr), Some(size)) => {
                                            read_pointer_array(vaddr, size)
                                        }
                                        _ => alloc::vec::Vec::new(),
                                    },
                                    fini_array: match (
                                        dynamic.dt_fini_array,
                                        dynamic.dt_fini_arraysz,
                                    ) {
                                        (Some(vaddr), Some(size)) => {
                                            read_pointer_array(vaddr, size)
                                        }
                                        _ => alloc::vec::Vec::new(),
                                    },
                                    fini: dynamic.dt_fini.map(runtime_addr),
                                };
                                process.set_runtime_hooks(runtime_hooks.clone());

                                // closures to read/write process memory at virtual addresses
                                let mappings = prepared.mappings.clone();
                                let read_u64_at = |v: u64| -> Option<u64> {
                                    // validate against mappings
                                    for m in &mappings {
                                        if v >= m.start && v + 8 <= m.end {
                                            unsafe {
                                                let ptr = v as *const u64;
                                                return Some(core::ptr::read_unaligned(ptr));
                                            }
                                        }
                                    }
                                    None
                                };
                                let read_u32_at = |v: u64| -> Option<u32> {
                                    for m in &mappings {
                                        if v >= m.start && v + 4 <= m.end {
                                            unsafe {
                                                let ptr = v as *const u32;
                                                return Some(core::ptr::read_unaligned(ptr));
                                            }
                                        }
                                    }
                                    None
                                };
                                let write_u64_at = |v: u64, val: u64| {
                                    for m in &mappings {
                                        if v >= m.start && v + 8 <= m.end {
                                            unsafe {
                                                let ptr = v as *mut u64;
                                                core::ptr::write_unaligned(ptr, val);
                                            }
                                            return;
                                        }
                                    }
                                };
                                let write_bytes_at = |v: u64, data: &[u8]| {
                                    for m in &mappings {
                                        if v >= m.start && v + (data.len() as u64) <= m.end {
                                            unsafe {
                                                core::ptr::copy_nonoverlapping(
                                                    data.as_ptr(),
                                                    v as *mut u8,
                                                    data.len(),
                                                );
                                            }
                                            return;
                                        }
                                    }
                                };

                                let mapping_contains_range = |addr: u64, len: u64| -> bool {
                                    let Some(end) = addr.checked_add(len) else {
                                        return false;
                                    };
                                    mappings
                                        .iter()
                                        .any(|m| addr >= m.start && end <= m.end)
                                };
                                let relocation_table_is_sane = |table: &crate::kernel::dynamic_linker::elf_dynamic::RelocationTable| -> bool {
                                    if table.entries.len() > 1_000_000 {
                                        return false;
                                    }
                                    table.entries.iter().all(|r| {
                                        let sym_idx = (r.info >> 32) as usize;
                                        let target = runtime_base.wrapping_add(r.offset);
                                        (sym_idx == 0 || sym_idx < symtab.symbols.len())
                                            && mapping_contains_range(target, 8)
                                    })
                                };

                                // resolver: prefer versioned matches from loader when local symbol
                                // has a version requirement; otherwise fall back to loader or local.
                                let resolve = |name: &str| -> Option<u64> {
                                    // If local symbol declares a desired version, attempt versioned lookup first
                                    if let Some(local) = symtab.find_by_name(name) {
                                        if let Some(vn) = local.vers_name.as_deref() {
                                            if let Some(s) =
                                                loader.find_symbol_versioned(name, Some(vn))
                                            {
                                                return Some(s.addr);
                                            }
                                        }
                                    }
                                    if let Some(s) = loader.find_symbol(name) {
                                        return Some(s.addr);
                                    }
                                    symtab
                                        .find_by_name(name)
                                        .map(|s| s.addr.wrapping_add(prepared.load_plan.aslr_base))
                                };

                                // process RELA then REL if present
                                if let Some(rela_vaddr) = dynamic.dt_rela {
                                    if let Some(rela_off) = vaddr_to_off(rela_vaddr) {
                                        if dynamic.dt_relaent.is_some_and(|v| v != 24) {
                                            continue;
                                        }
                                        let relasz = dynamic.dt_relasz.unwrap_or(0) as usize;
                                        if (relasz % 24) != 0 {
                                            continue;
                                        }
                                        let rel_count = relasz / 24;
                                        if let Some(rel_table) = crate::kernel::dynamic_linker::elf_dynamic::RelocationTable::parse(image, rela_off as u64, rel_count, crate::kernel::dynamic_linker::elf_dynamic::RelocationType::Rela) {
                                            if relocation_table_is_sane(&rel_table) {
                                                crate::kernel::dynamic_linker::elf_dynamic::process_relocations_inplace(
                                                    &rel_table,
                                                    runtime_base,
                                                    prepared.load_plan.tls_virtual_addr,
                                                    prepared.load_plan.tls_mem_size,
                                                    prepared.load_plan.tls_align,
                                                    &symtab,
                                                    &resolve,
                                                    &read_u64_at,
                                                    &read_u32_at,
                                                    &write_u64_at,
                                                    &write_bytes_at,
                                                );
                                            }
                                        }
                                    }
                                }
                                if let Some(rel_vaddr) = dynamic.dt_rel {
                                    if let Some(rel_off) = vaddr_to_off(rel_vaddr) {
                                        if dynamic.dt_relent.is_some_and(|v| v != 16) {
                                            continue;
                                        }
                                        let relsz = dynamic.dt_relsz.unwrap_or(0) as usize;
                                        if (relsz % 16) != 0 {
                                            continue;
                                        }
                                        let rel_count = relsz / 16;
                                        if let Some(rel_table) = crate::kernel::dynamic_linker::elf_dynamic::RelocationTable::parse(image, rel_off as u64, rel_count, crate::kernel::dynamic_linker::elf_dynamic::RelocationType::Rel) {
                                            if relocation_table_is_sane(&rel_table) {
                                                crate::kernel::dynamic_linker::elf_dynamic::process_relocations_inplace(
                                                    &rel_table,
                                                    runtime_base,
                                                    prepared.load_plan.tls_virtual_addr,
                                                    prepared.load_plan.tls_mem_size,
                                                    prepared.load_plan.tls_align,
                                                    &symtab,
                                                    &resolve,
                                                    &read_u64_at,
                                                    &read_u32_at,
                                                    &write_u64_at,
                                                    &write_bytes_at,
                                                );
                                            }
                                        }
                                    }
                                }

                                // Register PLT/JMP_SLOT entries for possible lazy binding
                                if let Some(jmp_vaddr) = dynamic.dt_jmprel {
                                    if let Some(jmp_off) = vaddr_to_off(jmp_vaddr) {
                                        let pltrelsz = dynamic.dt_pltrelsz.unwrap_or(0) as usize;
                                        let is_rela = match dynamic.dt_pltrel {
                                            Some(7) => true,
                                            Some(17) => false,
                                            Some(_) => continue,
                                            None => false,
                                        };
                                        let entry_size = if is_rela { 24 } else { 16 };
                                        let rel_count = if entry_size != 0 {
                                            if (pltrelsz % entry_size) != 0 {
                                                0
                                            } else {
                                            pltrelsz / entry_size
                                            }
                                        } else {
                                            0
                                        };
                                        if rel_count != 0 {
                                            let rel_type = if is_rela {
                                                crate::kernel::dynamic_linker::elf_dynamic::RelocationType::Rela
                                            } else {
                                                crate::kernel::dynamic_linker::elf_dynamic::RelocationType::Rel
                                            };
                                            if let Some(rel_table) = crate::kernel::dynamic_linker::elf_dynamic::RelocationTable::parse(image, jmp_off as u64, rel_count, rel_type) {
                                                if relocation_table_is_sane(&rel_table) {
                                                    for r in &rel_table.entries {
                                                        let rtype = (r.info & 0xffffffff) as u32;
                                                        let sym_idx = (r.info >> 32) as usize;
                                                        if rtype == crate::kernel::dynamic_linker::elf_dynamic::RelocTypeX86_64::JMP_SLOT as u32 {
                                                            if let Some(sym) = symtab.symbols.get(sym_idx) {
                                                                let slot_vaddr = runtime_base.wrapping_add(r.offset);
                                                                let is_ifunc = crate::kernel::symbol::SymbolTable::is_ifunc(sym);
                                                                let resolver_vaddr = runtime_base.wrapping_add(sym.addr);
                                                                loader.register_plt_slot(slot_vaddr, &sym.name, is_ifunc, resolver_vaddr);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                trampoline::install_lazy_plt_trampolines(
                                    process,
                                    &runtime_hooks,
                                    prepared,
                                    &mut loader,
                                    page_manager,
                                    frame_allocator,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
