use super::*;

#[path = "relocate/inplace.rs"]
mod inplace;
pub use inplace::process_relocations_inplace;

/// Process all relocations for an image, using the symbol table and base address
pub fn process_relocations(
    relocs: &RelocationTable,
    image: &mut [u8],
    base_addr: u64,
    tls_vaddr: u64,
    tls_mem_size: u64,
    tls_align: u64,
    symtab: &super::super::symbol::SymbolTable,
    resolve_symbol: &dyn Fn(&str) -> Option<u64>,
    vaddr_to_off: &dyn Fn(u64) -> Option<usize>,
) {
    fn tls_offsets(
        sym_addr: u64,
        tls_vaddr: u64,
        tls_mem_size: u64,
        tls_align: u64,
    ) -> Option<(u64, i64)> {
        if tls_vaddr == 0 || tls_mem_size == 0 || sym_addr < tls_vaddr {
            return None;
        }
        let dtpoff = sym_addr.checked_sub(tls_vaddr)?;
        let align = tls_align.max(1);
        let static_span = tls_mem_size.checked_add(align - 1)? / align * align;
        let tpoff = (dtpoff as i64).wrapping_sub(static_span as i64);
        Some((dtpoff, tpoff))
    }
    // helpers
    fn read_u64(image: &[u8], off: usize) -> Option<u64> {
        if off + 8 <= image.len() {
            let mut b = [0u8; 8];
            b.copy_from_slice(&image[off..off + 8]);
            Some(u64::from_le_bytes(b))
        } else {
            None
        }
    }
    fn read_u32(image: &[u8], off: usize) -> Option<u32> {
        if off + 4 <= image.len() {
            let mut b = [0u8; 4];
            b.copy_from_slice(&image[off..off + 4]);
            Some(u32::from_le_bytes(b))
        } else {
            None
        }
    }

    for rel in &relocs.entries {
        let reloc_type = (rel.info & 0xffffffff) as u32;
        let sym_idx = (rel.info >> 32) as usize;
        let reloc_vaddr = base_addr.wrapping_add(rel.offset);
        let reloc_off = match vaddr_to_off(reloc_vaddr) {
            Some(o) => o,
            None => continue,
        };

        let sym_addr = if sym_idx != 0 {
            symtab
                .symbols
                .get(sym_idx)
                .map(|s| resolve_symbol(&s.name).unwrap_or(s.addr))
                .unwrap_or(0)
        } else {
            0
        };

        // Determine addend: prefer explicit addend (RELA), otherwise read from image for REL
        let addend_u64 = rel.addend.or_else(|| {
            if matches!(relocs.rel_type, RelocationType::Rel) {
                read_u64(image, reloc_off)
            } else {
                None
            }
        });

        match reloc_type {
            x if x == RelocTypeX86_64::NONE as u32 => {}
            x if x == RelocTypeX86_64::_64 as u32 => {
                let a = addend_u64.unwrap_or(0);
                let val = sym_addr.wrapping_add(a);
                patch_u64(image, reloc_off, val);
            }
            x if x == RelocTypeX86_64::RELATIVE as u32 => {
                let a = addend_u64.unwrap_or(0);
                let val = base_addr.wrapping_add(a);
                patch_u64(image, reloc_off, val);
            }
            x if x == RelocTypeX86_64::GLOB_DAT as u32 || x == RelocTypeX86_64::JMP_SLOT as u32 => {
                // Write symbol address plus addend when provided (best-effort).
                let a = addend_u64.unwrap_or(0);
                let val = sym_addr.wrapping_add(a);
                klog_debug!("Applying GLOB_DAT/JMP_SLOT reloc: sym_idx={} sym_addr={:#x} addend={:#x} -> val={:#x}", sym_idx, sym_addr, a, val);
                patch_u64(image, reloc_off, val);
            }
            x if x == RelocTypeX86_64::PLT32 as u32 || x == RelocTypeX86_64::PC32 as u32 => {
                // 32-bit PC-relative: S + A - P
                let a32 = rel
                    .addend
                    .and_then(|v| Some(v as i64))
                    .or_else(|| {
                        if matches!(relocs.rel_type, RelocationType::Rel) {
                            read_u32(image, reloc_off).map(|v| v as i64)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                let place = reloc_vaddr as i64;
                let target = (sym_addr as i64).wrapping_add(a32);
                let diff = target.wrapping_sub(place);
                let rel32 = diff as i32;
                if reloc_off + 4 <= image.len() {
                    klog_debug!(
                        "Applying PLT32/PC32 reloc: sym_idx={} sym_addr={:#x} place={:#x} rel32={}",
                        sym_idx,
                        sym_addr,
                        place as u64,
                        rel32
                    );
                    image[reloc_off..reloc_off + 4].copy_from_slice(&rel32.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::GOT32 as u32 => {
                // 32-bit GOT absolute: write low 32 bits of S + A
                let a = addend_u64.unwrap_or(0) as u64;
                let val = sym_addr.wrapping_add(a) as u32;
                if reloc_off + 4 <= image.len() {
                    image[reloc_off..reloc_off + 4].copy_from_slice(&val.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::GOTPCREL as u32 => {
                // 32-bit PC-relative to GOT: S + A - P
                let a32 = rel
                    .addend
                    .and_then(|v| Some(v as i64))
                    .or_else(|| {
                        if matches!(relocs.rel_type, RelocationType::Rel) {
                            read_u32(image, reloc_off).map(|v| v as i64)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                let place = reloc_vaddr as i64;
                let target = (sym_addr as i64).wrapping_add(a32);
                let rel32 = (target.wrapping_sub(place)) as i32;
                if reloc_off + 4 <= image.len() {
                    klog_debug!(
                        "Applying GOTPCREL reloc: sym_idx={} sym_addr={:#x} place={:#x} rel32={}",
                        sym_idx,
                        sym_addr,
                        reloc_vaddr,
                        rel32
                    );
                    image[reloc_off..reloc_off + 4].copy_from_slice(&rel32.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::GOTPCRELX as u32 => {
                // GOTPCRELX: 32-bit PC-relative to GOT-like target. Best-effort similar to GOTPCREL.
                let a32 = rel
                    .addend
                    .and_then(|v| Some(v as i64))
                    .or_else(|| {
                        if matches!(relocs.rel_type, RelocationType::Rel) {
                            read_u32(image, reloc_off).map(|v| v as i64)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                let place = reloc_vaddr as i64;
                let target = (sym_addr as i64).wrapping_add(a32);
                // Compute difference with wrapping semantics then truncate to i32 (signed wrap)
                let diff = target.wrapping_sub(place);
                let rel32 = diff as i32;
                if reloc_off + 4 <= image.len() {
                    klog_debug!(
                        "Applying GOTPCRELX reloc: sym_idx={} sym_addr={:#x} place={:#x} rel32={}",
                        sym_idx,
                        sym_addr,
                        place as u64,
                        rel32
                    );
                    image[reloc_off..reloc_off + 4].copy_from_slice(&rel32.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::TLSGD as u32 || x == RelocTypeX86_64::TLSLD as u32 => {
                // TLS GD/LD: best-effort — write symbol address + addend where possible.
                if sym_idx != 0 {
                    if let Some(s) = symtab.symbols.get(sym_idx) {
                        let a = addend_u64.unwrap_or(0);
                        let val = if let Some((dtpoff, _)) =
                            tls_offsets(s.addr, tls_vaddr, tls_mem_size, tls_align)
                        {
                            dtpoff.wrapping_add(a)
                        } else {
                            s.addr.wrapping_add(a)
                        };
                        patch_u64(image, reloc_off, val);
                        continue;
                    }
                }
                let a = addend_u64.unwrap_or(0);
                patch_u64(image, reloc_off, a);
            }
            x if x == RelocTypeX86_64::_32 as u32 => {
                // 32-bit absolute: S + A (truncate to 32 bits)
                let a = addend_u64.unwrap_or(0) as i64;
                let val = (sym_addr as i64).wrapping_add(a) as i32 as u32;
                if reloc_off + 4 <= image.len() {
                    image[reloc_off..reloc_off + 4].copy_from_slice(&val.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::TPOFF32 as u32 || x == RelocTypeX86_64::DTPOFF32 as u32 => {
                // 32-bit TLS offset: write signed 32-bit S + A when possible, else addend
                let a = addend_u64.unwrap_or(0) as i64;
                let sval = if x == RelocTypeX86_64::TPOFF32 as u32 {
                    tls_offsets(sym_addr, tls_vaddr, tls_mem_size, tls_align)
                        .map(|(_, tpoff)| tpoff.wrapping_add(a))
                        .unwrap_or((sym_addr as i64).wrapping_add(a))
                } else {
                    tls_offsets(sym_addr, tls_vaddr, tls_mem_size, tls_align)
                        .map(|(dtpoff, _)| (dtpoff as i64).wrapping_add(a))
                        .unwrap_or((sym_addr as i64).wrapping_add(a))
                } as i32;
                if reloc_off + 4 <= image.len() {
                    image[reloc_off..reloc_off + 4].copy_from_slice(&sval.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::_32S as u32 => {
                // 32-bit signed absolute: S + A (signed truncation)
                let a = addend_u64.unwrap_or(0) as i64;
                let sval = (sym_addr as i64).wrapping_add(a) as i32;
                if reloc_off + 4 <= image.len() {
                    image[reloc_off..reloc_off + 4].copy_from_slice(&sval.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::TPOFF64 as u32
                || x == RelocTypeX86_64::DTPOFF64 as u32
                || x == RelocTypeX86_64::DTPMOD64 as u32
                || x == RelocTypeX86_64::TPOFF32 as u32
                || x == RelocTypeX86_64::DTPOFF32 as u32
                || x == RelocTypeX86_64::GOTTPOFF as u32 =>
            {
                // TLS relocations with a basic local-exec style layout.
                if sym_idx != 0 {
                    if let Some(s) = symtab.symbols.get(sym_idx) {
                        let a = addend_u64.unwrap_or(0);
                        let val = if x == RelocTypeX86_64::DTPMOD64 as u32 {
                            1
                        } else if x == RelocTypeX86_64::DTPOFF64 as u32 {
                            tls_offsets(s.addr, tls_vaddr, tls_mem_size, tls_align)
                                .map(|(dtpoff, _)| dtpoff.wrapping_add(a))
                                .unwrap_or_else(|| s.addr.wrapping_add(a))
                        } else {
                            tls_offsets(s.addr, tls_vaddr, tls_mem_size, tls_align)
                                .map(|(_, tpoff)| (tpoff as u64).wrapping_add(a))
                                .unwrap_or_else(|| s.addr.wrapping_add(a))
                        };
                        patch_u64(image, reloc_off, val);
                        continue;
                    }
                }
                let a = addend_u64.unwrap_or(0);
                patch_u64(image, reloc_off, a);
            }
            x if x == RelocTypeX86_64::GOT32 as u32 => {
                // 32-bit GOT absolute: write low 32 bits of S + A
                let a = addend_u64.unwrap_or(0) as u64;
                let val = sym_addr.wrapping_add(a) as u32;
                if reloc_off + 4 <= image.len() {
                    image[reloc_off..reloc_off + 4].copy_from_slice(&val.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::GOTPCREL as u32 => {
                // 32-bit PC-relative to GOT: S + A - P
                let a32 = rel
                    .addend
                    .and_then(|v| Some(v as i64))
                    .or_else(|| {
                        if matches!(relocs.rel_type, RelocationType::Rel) {
                            read_u32(image, reloc_off).map(|v| v as i64)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                let place = reloc_vaddr as i64;
                let target = (sym_addr as i64).wrapping_add(a32);
                let rel32 = (target.wrapping_sub(place)) as i32;
                if reloc_off + 4 <= image.len() {
                    image[reloc_off..reloc_off + 4].copy_from_slice(&rel32.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::TPOFF64 as u32
                || x == RelocTypeX86_64::DTPOFF64 as u32
                || x == RelocTypeX86_64::DTPMOD64 as u32
                || x == RelocTypeX86_64::TPOFF32 as u32
                || x == RelocTypeX86_64::DTPOFF32 as u32
                || x == RelocTypeX86_64::GOTTPOFF as u32 =>
            {
                // TLS relocations: require TLS layout info. Best-effort: set to addend or zero.
                let a = addend_u64.unwrap_or(0);
                patch_u64(image, reloc_off, a);
            }
            x if x == RelocTypeX86_64::IRELATIVE as u32 => {
                // Best-effort: prefer a previously-resolved symbol value, else fall back to B + A
                if sym_idx != 0 {
                    if let Some(s) = symtab.symbols.get(sym_idx) {
                        let resolved = resolve_symbol(&s.name).unwrap_or(s.addr);
                        patch_u64(image, reloc_off, resolved);
                        continue;
                    }
                }
                if let Some(a) = addend_u64 {
                    let val = base_addr.wrapping_add(a);
                    patch_u64(image, reloc_off, val);
                }
            }
            x if x == RelocTypeX86_64::GOT32 as u32 => {
                let a = addend_u64.unwrap_or(0) as u64;
                let val = sym_addr.wrapping_add(a) as u32;
                if reloc_off + 4 <= image.len() {
                    image[reloc_off..reloc_off + 4].copy_from_slice(&val.to_le_bytes());
                }
            }
            x if x == RelocTypeX86_64::COPY as u32 => {
                // COPY: copy data from the defining object's address into this image.
                if let Some(sym) = symtab.symbols.get(sym_idx) {
                    let size = sym.size as usize;
                    if size > 0 {
                        // Prefer resolved symbol address if available
                        let src_vaddr = resolve_symbol(&sym.name).unwrap_or(sym.addr);
                        if let Some(src_off) = vaddr_to_off(src_vaddr) {
                            if src_off + size <= image.len() && reloc_off + size <= image.len() {
                                let src = image[src_off..src_off + size].to_vec();
                                image[reloc_off..reloc_off + size].copy_from_slice(&src);
                            }
                        }
                    }
                }
            }
            _ => {
                klog_warn!(
                    "Unhandled relocation type {} at offset {:#x} (sym_idx={})",
                    reloc_type,
                    reloc_vaddr,
                    sym_idx
                );
            }
        }
    }
}

fn patch_u64(image: &mut [u8], off: usize, val: u64) {
    if off + 8 <= image.len() {
        image[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }
}
