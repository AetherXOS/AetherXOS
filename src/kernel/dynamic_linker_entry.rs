use super::helpers::{
    estimate_image_window_bytes, is_supported_elf, read_dynstr_entry, split_search_paths,
    DEFAULT_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES, ELF64_SYM_SIZE_BYTES, MAX_HEURISTIC_SYMBOL_COUNT,
};
use super::{elf_dynamic, so_loader, symbol, ElfFile, Type};

pub fn dynamic_linker_entry(entry_addr: u64, _auxv: &[usize]) {
    let image_window = estimate_image_window_bytes(entry_addr)
        .unwrap_or(DEFAULT_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES);
    let image = unsafe { core::slice::from_raw_parts(entry_addr as *const u8, image_window) };

    let elf = match ElfFile::new(image) {
        Ok(e) => e,
        Err(_) => return,
    };

    if !is_supported_elf(&elf) {
        return;
    }

    let mut dyn_off = None;
    let mut dyn_size = None;
    for ph in elf.program_iter() {
        if let Ok(Type::Dynamic) = ph.get_type() {
            dyn_off = Some(ph.offset() as usize);
            dyn_size = Some(ph.file_size() as usize);
            break;
        }
    }
    let (dyn_off, dyn_size) = match (dyn_off, dyn_size) {
        (Some(o), Some(s)) => (o, s),
        _ => return,
    };

    let dyn_count = dyn_size / 16;
    let dynamic = match elf_dynamic::DynamicSection::parse(image, dyn_off as u64, dyn_count) {
        Some(d) => d,
        None => return,
    };

    let vaddr_to_offset = |vaddr: u64| -> Option<usize> {
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

    let strtab_file_off = dynamic.dt_strtab.and_then(vaddr_to_offset);

    let mut needed = alloc::vec::Vec::new();
    if let Some(strtab_off) = strtab_file_off {
        for name_off in &dynamic.dt_needed {
            if let Some(name) = read_dynstr_entry(image, strtab_off, *name_off) {
                needed.push(name);
            }
        }
    }

    let mut search_paths = alloc::vec::Vec::new();
    if let Some(strtab_off) = strtab_file_off {
        if let Some(runpath_off) = dynamic.dt_runpath {
            if let Some(runpath) = read_dynstr_entry(image, strtab_off, runpath_off) {
                search_paths.extend(split_search_paths(&runpath));
            }
        } else if let Some(rpath_off) = dynamic.dt_rpath {
            if let Some(rpath) = read_dynstr_entry(image, strtab_off, rpath_off) {
                search_paths.extend(split_search_paths(&rpath));
            }
        }
    }

    let mut loader = so_loader::SharedObjectLoader::with_search_paths(&search_paths);
    loader.load_needed(&needed);

    let mut sym_count = 0usize;
    if let Some(hash_vaddr) = dynamic.dt_hash {
        if let Some(hash_off) = vaddr_to_offset(hash_vaddr) {
            sym_count = elf_dynamic::parse_sysv_hash_nchain(image, hash_off).unwrap_or(0);
        }
    }
    if sym_count == 0 {
        if let Some(gnu_hash_vaddr) = dynamic.dt_gnu_hash {
            if let Some(gnu_hash_off) = vaddr_to_offset(gnu_hash_vaddr) {
                sym_count =
                    elf_dynamic::parse_gnu_hash_symbol_count(image, gnu_hash_off).unwrap_or(0);
            }
        }
    }

    let symtab_file_off = match dynamic.dt_symtab.and_then(vaddr_to_offset) {
        Some(o) => o,
        None => return,
    };
    let strtab_file_off = match dynamic.dt_strtab.and_then(vaddr_to_offset) {
        Some(o) => o,
        None => return,
    };

    if sym_count == 0 {
        let sym_entry_size = dynamic.dt_syment.unwrap_or(ELF64_SYM_SIZE_BYTES as u64) as usize;
        if sym_entry_size == 0 {
            return;
        }
        // Without hash tables, safest heuristic is the span between .dynsym and .dynstr.
        // If section ordering is unusual, fall back to a DT_STRSZ-based upper bound.
        if symtab_file_off < strtab_file_off {
            sym_count = (strtab_file_off.saturating_sub(symtab_file_off)) / sym_entry_size;
        } else {
            let symtab_end = dynamic
                .dt_strsz
                .and_then(|strsz| strtab_file_off.checked_add(strsz as usize))
                .unwrap_or(image.len())
                .min(image.len());
            if symtab_end > symtab_file_off {
                sym_count = (symtab_end.saturating_sub(symtab_file_off)) / sym_entry_size;
            }
        }
        if sym_count > MAX_HEURISTIC_SYMBOL_COUNT {
            sym_count = 0;
        }
    }

    if sym_count == 0 {
        return;
    }

    let versym_off = dynamic.dt_versym.and_then(vaddr_to_offset);
    let symtab = match symbol::SymbolTable::parse(
        image,
        strtab_file_off as u64,
        symtab_file_off as u64,
        sym_count,
        versym_off.map(|o| o as u64),
        None,
        None,
    ) {
        Some(s) => s,
        None => return,
    };

    let mut tls_vaddr = 0u64;
    let mut tls_mem_size = 0u64;
    let mut tls_align = 1u64;
    for ph in elf.program_iter() {
        if let Ok(Type::Tls) = ph.get_type() {
            tls_vaddr = ph.virtual_addr();
            tls_mem_size = ph.mem_size();
            tls_align = ph.align().max(1);
            break;
        }
    }

    if let Some(rela_vaddr) = dynamic.dt_rela {
        if let Some(rela_off) = vaddr_to_offset(rela_vaddr) {
            let relasz = dynamic.dt_relasz.unwrap_or(0) as usize;
            let rel_count = relasz / 24;
            if let Some(rel_table) = elf_dynamic::RelocationTable::parse(
                image,
                rela_off as u64,
                rel_count,
                elf_dynamic::RelocationType::Rela,
            ) {
                let resolve = |name: &str| -> Option<u64> {
                    if let Some(sym) = loader.find_symbol(name) {
                        return Some(sym.addr);
                    }
                    symtab.find_by_name(name).map(|s| s.addr)
                };

                let mut image_base = u64::MAX;
                for ph in elf.program_iter() {
                    if let Ok(Type::Load) = ph.get_type() {
                        image_base = image_base.min(ph.virtual_addr());
                    }
                }
                if image_base == u64::MAX {
                    image_base = 0;
                }

                let mut image_owned = alloc::vec::Vec::from(image);
                let vaddr_to_off = |v: u64| -> Option<usize> {
                    for ph in elf.program_iter() {
                        if let Ok(Type::Load) = ph.get_type() {
                            let va = ph.virtual_addr();
                            let memsz = ph.mem_size();
                            if v >= va && v < va + memsz {
                                let off = ph.offset() + (v - va);
                                return usize::try_from(off).ok();
                            }
                        }
                    }
                    None
                };
                elf_dynamic::process_relocations(
                    &rel_table,
                    &mut image_owned,
                    image_base,
                    tls_vaddr,
                    tls_mem_size,
                    tls_align,
                    &symtab,
                    &resolve,
                    &vaddr_to_off,
                );
            }
        }
    }
}
