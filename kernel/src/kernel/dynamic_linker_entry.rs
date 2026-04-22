use super::helpers::{
    estimate_image_window_bytes, is_supported_elf, read_dynstr_entry, resolve_runtime_search_paths,
    DEFAULT_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES, ELF64_SYM_SIZE_BYTES,
    MAX_HEURISTIC_SYMBOL_COUNT,
};
use super::{elf_dynamic, so_loader, symbol, ElfFile, Type};

const AT_NULL: usize = crate::kernel::syscalls::syscalls_consts::linux::AT_NULL;
const AT_BASE: usize = 7;
const AT_ENTRY: usize = crate::kernel::syscalls::syscalls_consts::linux::AT_ENTRY;
const AT_PHDR: usize = 3;
const AT_PHENT: usize = 4;
const AT_PHNUM: usize = 5;
const AT_SYSINFO_EHDR: usize = 33;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AuxvContractSnapshot {
    at_base: Option<usize>,
    at_entry: Option<usize>,
    at_phdr: Option<usize>,
    at_phent: Option<usize>,
    at_phnum: Option<usize>,
    at_sysinfo_ehdr: Option<usize>,
}

fn program_header_addr(elf: &ElfFile<'_>) -> Option<usize> {
    for ph in elf.program_iter() {
        if let Ok(Type::Phdr) = ph.get_type() {
            return usize::try_from(ph.virtual_addr()).ok();
        }
    }

    let phoff = usize::try_from(elf.header.pt2.ph_offset()).ok()?;
    for ph in elf.program_iter() {
        if let Ok(Type::Load) = ph.get_type() {
            let seg_off = usize::try_from(ph.offset()).ok()?;
            let seg_end = seg_off.checked_add(usize::try_from(ph.file_size()).ok()?)?;
            if phoff >= seg_off && phoff < seg_end {
                let delta = phoff - seg_off;
                return usize::try_from(ph.virtual_addr()).ok()?.checked_add(delta);
            }
        }
    }

    None
}

fn parse_auxv_contract(auxv: &[usize]) -> Option<AuxvContractSnapshot> {
    if auxv.len() % 2 != 0 {
        return None;
    }

    let mut snapshot = AuxvContractSnapshot {
        at_base: None,
        at_entry: None,
        at_phdr: None,
        at_phent: None,
        at_phnum: None,
        at_sysinfo_ehdr: None,
    };

    for pair in auxv.chunks_exact(2) {
        let key = pair[0];
        let value = pair[1];
        if key == AT_NULL {
            break;
        }
        match key {
            AT_BASE => snapshot.at_base = Some(value),
            AT_ENTRY => snapshot.at_entry = Some(value),
            AT_PHDR => snapshot.at_phdr = Some(value),
            AT_PHENT => snapshot.at_phent = Some(value),
            AT_PHNUM => snapshot.at_phnum = Some(value),
            AT_SYSINFO_EHDR => snapshot.at_sysinfo_ehdr = Some(value),
            _ => {}
        }
    }

    Some(snapshot)
}

fn validate_auxv_contract(elf: &ElfFile<'_>, entry_addr: u64, auxv: &[usize]) -> bool {
    let Some(snapshot) = parse_auxv_contract(auxv) else {
        return false;
    };

    let at_base = match snapshot.at_base {
        Some(value) if value != 0 => value,
        _ => return false,
    };
    let at_entry = match snapshot.at_entry {
        Some(value) if value != 0 => value,
        _ => return false,
    };
    let at_phdr = match snapshot.at_phdr {
        Some(value) if value != 0 => value,
        _ => return false,
    };
    let at_phent = match snapshot.at_phent {
        Some(value) if (16..=4096).contains(&value) => value,
        _ => return false,
    };
    let at_phnum = match snapshot.at_phnum {
        Some(value) if value != 0 && value <= 4096 => value,
        _ => return false,
    };
    if snapshot.at_sysinfo_ehdr == Some(0) {
        return false;
    }

    if at_base == entry_addr as usize {
        return false;
    }
    if at_base & (0x1000 - 1) != 0 || snapshot.at_sysinfo_ehdr.is_some_and(|value| value & (0x1000 - 1) != 0) {
        return false;
    }
    if at_phdr < at_base {
        return false;
    }

    let Some(actual_phdr_addr) = program_header_addr(elf) else {
        return false;
    };
    if at_phdr != actual_phdr_addr {
        return false;
    }
    if at_phent != usize::try_from(elf.header.pt2.ph_entry_size()).ok().unwrap_or(0) {
        return false;
    }
    if at_phnum != usize::try_from(elf.header.pt2.ph_count()).ok().unwrap_or(0) {
        return false;
    }
    at_entry != 0
}

pub fn dynamic_linker_entry(entry_addr: u64, auxv: &[usize]) {
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

    if !validate_auxv_contract(&elf, entry_addr, auxv) {
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

    let mut runpath_str: Option<alloc::string::String> = None;
    let mut rpath_str: Option<alloc::string::String> = None;
    if let Some(strtab_off) = strtab_file_off {
        if let Some(runpath_off) = dynamic.dt_runpath {
            if let Some(runpath) = read_dynstr_entry(image, strtab_off, runpath_off) {
                runpath_str = Some(runpath);
            }
        }
        if let Some(rpath_off) = dynamic.dt_rpath {
            if let Some(rpath) = read_dynstr_entry(image, strtab_off, rpath_off) {
                rpath_str = Some(rpath);
            }
        }
    }
    let search_paths = resolve_runtime_search_paths(runpath_str.as_deref(), rpath_str.as_deref());

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

#[cfg(test)]
mod tests {
    use super::*;

    fn loader_image_with_phdr() -> alloc::vec::Vec<u8> {
        const ELF_HEADER_SIZE: usize = 64;
        const PHDR_SIZE: usize = 56;
        const PHDR_OFFSET: usize = ELF_HEADER_SIZE;
        let mut image = alloc::vec![0u8; 512];

        image[0..4].copy_from_slice(b"\x7FELF");
        image[4] = 2;
        image[5] = 1;
        image[6] = 1;
        image[16..18].copy_from_slice(&2u16.to_le_bytes());
        image[18..20].copy_from_slice(&62u16.to_le_bytes());
        image[20..24].copy_from_slice(&1u32.to_le_bytes());
        image[24..32].copy_from_slice(&0x401000u64.to_le_bytes());
        image[32..40].copy_from_slice(&(PHDR_OFFSET as u64).to_le_bytes());
        image[52..54].copy_from_slice(&(ELF_HEADER_SIZE as u16).to_le_bytes());
        image[54..56].copy_from_slice(&(PHDR_SIZE as u16).to_le_bytes());
        image[56..58].copy_from_slice(&2u16.to_le_bytes());

        image[PHDR_OFFSET..PHDR_OFFSET + 4].copy_from_slice(&1u32.to_le_bytes());
        image[PHDR_OFFSET + 4..PHDR_OFFSET + 8].copy_from_slice(&5u32.to_le_bytes());
        image[PHDR_OFFSET + 8..PHDR_OFFSET + 16].copy_from_slice(&0u64.to_le_bytes());
        image[PHDR_OFFSET + 16..PHDR_OFFSET + 24].copy_from_slice(&0x400000u64.to_le_bytes());
        image[PHDR_OFFSET + 24..PHDR_OFFSET + 32].copy_from_slice(&0x400000u64.to_le_bytes());
        image[PHDR_OFFSET + 32..PHDR_OFFSET + 40].copy_from_slice(&512u64.to_le_bytes());
        image[PHDR_OFFSET + 40..PHDR_OFFSET + 48].copy_from_slice(&512u64.to_le_bytes());
        image[PHDR_OFFSET + 48..PHDR_OFFSET + 56].copy_from_slice(&0x1000u64.to_le_bytes());

        let phdr_offset = PHDR_OFFSET + PHDR_SIZE;
        image[phdr_offset..phdr_offset + 4].copy_from_slice(&6u32.to_le_bytes());
        image[phdr_offset + 4..phdr_offset + 8].copy_from_slice(&4u32.to_le_bytes());
        image[phdr_offset + 8..phdr_offset + 16].copy_from_slice(&(PHDR_OFFSET as u64).to_le_bytes());
        image[phdr_offset + 16..phdr_offset + 24].copy_from_slice(&0x400040u64.to_le_bytes());
        image[phdr_offset + 24..phdr_offset + 32].copy_from_slice(&0x400040u64.to_le_bytes());
        image[phdr_offset + 32..phdr_offset + 40].copy_from_slice(&(PHDR_SIZE as u64).to_le_bytes());
        image[phdr_offset + 40..phdr_offset + 48].copy_from_slice(&(PHDR_SIZE as u64).to_le_bytes());
        image[phdr_offset + 48..phdr_offset + 56].copy_from_slice(&8u64.to_le_bytes());

        image
    }

    #[test_case]
    fn parse_auxv_contract_rejects_odd_length() {
        assert!(parse_auxv_contract(&[AT_BASE, 0x1000, AT_ENTRY]).is_none());
    }

    #[test_case]
    fn validate_auxv_contract_rejects_missing_required_entries() {
        let image = loader_image_with_phdr();
        let elf = ElfFile::new(&image).expect("elf");
        assert!(!validate_auxv_contract(&elf, 0x2000, &[AT_BASE, 0x3000, AT_ENTRY, 0x4000]));
    }

    #[test_case]
    fn validate_auxv_contract_accepts_baseline_handoff_metadata() {
        let image = loader_image_with_phdr();
        let elf = ElfFile::new(&image).expect("elf");
        let auxv = [
            AT_BASE,
            0x5000,
            AT_ENTRY,
            0x9000,
            AT_PHDR,
            0x400040,
            AT_PHENT,
            56,
            AT_PHNUM,
            2,
            AT_SYSINFO_EHDR,
            0xA000,
        ];
        assert!(validate_auxv_contract(&elf, 0x7000, &auxv));
    }

    #[test_case]
    fn validate_auxv_contract_rejects_misaligned_sysinfo_mapping() {
        let image = loader_image_with_phdr();
        let elf = ElfFile::new(&image).expect("elf");
        let auxv = [
            AT_BASE,
            0x5000,
            AT_ENTRY,
            0x9000,
            AT_PHDR,
            0x400040,
            AT_PHENT,
            56,
            AT_PHNUM,
            2,
            AT_SYSINFO_EHDR,
            0xA123,
        ];

        assert!(!validate_auxv_contract(&elf, 0x7000, &auxv));
    }
}
