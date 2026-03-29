use super::*;

/// Apply relocations directly against a live address space using callbacks.
pub fn process_relocations_inplace(
    relocs: &RelocationTable,
    base_addr: u64,
    tls_vaddr: u64,
    tls_mem_size: u64,
    tls_align: u64,
    symtab: &super::super::super::symbol::SymbolTable,
    resolve_symbol: &dyn Fn(&str) -> Option<u64>,
    read_u64_at: &dyn Fn(u64) -> Option<u64>,
    read_u32_at: &dyn Fn(u64) -> Option<u32>,
    write_u64_at: &dyn Fn(u64, u64),
    write_bytes_at: &dyn Fn(u64, &[u8]),
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
    for rel in &relocs.entries {
        let reloc_type = (rel.info & 0xffffffff) as u32;
        let sym_idx = (rel.info >> 32) as usize;
        let reloc_vaddr = base_addr.wrapping_add(rel.offset);

        let sym_addr = if sym_idx != 0 {
            symtab
                .symbols
                .get(sym_idx)
                .map(|s| resolve_symbol(&s.name).unwrap_or(s.addr))
                .unwrap_or(0)
        } else {
            0
        };

        // Determine addend: prefer explicit addend (RELA), otherwise read from target memory for REL
        let addend_u64 = rel.addend.or_else(|| read_u64_at(reloc_vaddr));

        match reloc_type {
            x if x == RelocTypeX86_64::NONE as u32 => {}
            x if x == RelocTypeX86_64::_64 as u32 => {
                let a = addend_u64.unwrap_or(0);
                let val = sym_addr.wrapping_add(a);
                write_u64_at(reloc_vaddr, val);
            }
            x if x == RelocTypeX86_64::RELATIVE as u32 => {
                let a = addend_u64.unwrap_or(0);
                let val = base_addr.wrapping_add(a);
                write_u64_at(reloc_vaddr, val);
            }
            x if x == RelocTypeX86_64::GLOB_DAT as u32 || x == RelocTypeX86_64::JMP_SLOT as u32 => {
                let a = addend_u64.unwrap_or(0);
                let val = sym_addr.wrapping_add(a);
                klog_debug!("Applying GLOB_DAT/JMP_SLOT reloc (in-place): sym_idx={} sym_addr={:#x} addend={:#x} -> val={:#x}", sym_idx, sym_addr, a, val);
                write_u64_at(reloc_vaddr, val);
            }
            x if x == RelocTypeX86_64::PLT32 as u32 || x == RelocTypeX86_64::PC32 as u32 => {
                // 32-bit PC-relative: S + A - P
                let a32 = rel
                    .addend
                    .and_then(|v| Some(v as i64))
                    .or_else(|| read_u32_at(reloc_vaddr).map(|v| v as i64))
                    .unwrap_or(0);
                let place = reloc_vaddr as i64;
                let target = (sym_addr as i64).wrapping_add(a32);
                let diff = target.wrapping_sub(place);
                let rel32 = diff as i32;
                klog_debug!("Applying PLT32/PC32 reloc (in-place): sym_idx={} sym_addr={:#x} place={:#x} rel32={}", sym_idx, sym_addr, place as u64, rel32);
                write_bytes_at(reloc_vaddr, &rel32.to_le_bytes());
            }
            x if x == RelocTypeX86_64::GOT32 as u32 => {
                let a = addend_u64.unwrap_or(0) as u64;
                let val = sym_addr.wrapping_add(a) as u32;
                write_bytes_at(reloc_vaddr, &val.to_le_bytes());
            }
            x if x == RelocTypeX86_64::GOTPCREL as u32 => {
                let a32 = rel
                    .addend
                    .and_then(|v| Some(v as i64))
                    .or_else(|| read_u32_at(reloc_vaddr).map(|v| v as i64))
                    .unwrap_or(0);
                let place = reloc_vaddr as i64;
                let target = (sym_addr as i64).wrapping_add(a32);
                let rel32 = (target.wrapping_sub(place)) as i32;
                klog_debug!("Applying GOTPCREL reloc (in-place): sym_idx={} sym_addr={:#x} place={:#x} rel32={}", sym_idx, sym_addr, reloc_vaddr, rel32);
                write_bytes_at(reloc_vaddr, &rel32.to_le_bytes());
            }
            x if x == RelocTypeX86_64::GOTPCRELX as u32 => {
                // GOTPCRELX: attempt same PC-relative computation as GOTPCREL
                let a32 = rel
                    .addend
                    .and_then(|v| Some(v as i64))
                    .or_else(|| read_u32_at(reloc_vaddr).map(|v| v as i64))
                    .unwrap_or(0);
                let place = reloc_vaddr as i64;
                let target = (sym_addr as i64).wrapping_add(a32);
                let diff = target.wrapping_sub(place);
                let rel32 = diff as i32;
                klog_debug!("Applying GOTPCRELX reloc (in-place): sym_idx={} sym_addr={:#x} place={:#x} rel32={}", sym_idx, sym_addr, place as u64, rel32);
                write_bytes_at(reloc_vaddr, &rel32.to_le_bytes());
            }
            x if x == RelocTypeX86_64::TLSGD as u32 || x == RelocTypeX86_64::TLSLD as u32 => {
                // TLS GD/LD: best-effort - write symbol address + addend when available.
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
                        write_u64_at(reloc_vaddr, val);
                        continue;
                    }
                }
                if let Some(a) = addend_u64 {
                    write_u64_at(reloc_vaddr, a);
                } else {
                    write_u64_at(reloc_vaddr, 0);
                }
            }
            x if x == RelocTypeX86_64::_32 as u32 => {
                // 32-bit absolute: S + A (truncate)
                let a = addend_u64.unwrap_or(0) as i64;
                let val = (sym_addr as i64).wrapping_add(a) as i32 as u32;
                write_bytes_at(reloc_vaddr, &val.to_le_bytes());
            }
            x if x == RelocTypeX86_64::TPOFF32 as u32 || x == RelocTypeX86_64::DTPOFF32 as u32 => {
                // 32-bit TLS offset in-place: sign-truncate S + A
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
                write_bytes_at(reloc_vaddr, &sval.to_le_bytes());
            }
            x if x == RelocTypeX86_64::_32S as u32 => {
                // 32-bit signed absolute: S + A
                let a = addend_u64.unwrap_or(0) as i64;
                let sval = (sym_addr as i64).wrapping_add(a) as i32;
                write_bytes_at(reloc_vaddr, &sval.to_le_bytes());
            }
            x if x == RelocTypeX86_64::TPOFF64 as u32
                || x == RelocTypeX86_64::DTPOFF64 as u32
                || x == RelocTypeX86_64::DTPMOD64 as u32
                || x == RelocTypeX86_64::TPOFF32 as u32
                || x == RelocTypeX86_64::DTPOFF32 as u32
                || x == RelocTypeX86_64::GOTTPOFF as u32 =>
            {
                // TLS relocations using a basic local-exec style TLS layout.
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
                        write_u64_at(reloc_vaddr, val);
                        continue;
                    }
                }
                if let Some(a) = addend_u64 {
                    write_u64_at(reloc_vaddr, a);
                } else {
                    write_u64_at(reloc_vaddr, 0);
                }
            }
            x if x == RelocTypeX86_64::IRELATIVE as u32 => {
                // IRELATIVE: cannot safely execute user resolver from kernel; prefer resolved symbol or B + A.
                if sym_idx != 0 {
                    if let Some(s) = symtab.symbols.get(sym_idx) {
                        let resolved = resolve_symbol(&s.name).unwrap_or(s.addr);
                        write_u64_at(reloc_vaddr, resolved);
                        continue;
                    }
                }
                if let Some(a) = addend_u64 {
                    let val = base_addr.wrapping_add(a);
                    write_u64_at(reloc_vaddr, val);
                }
            }
            x if x == RelocTypeX86_64::COPY as u32 => {
                if let Some(sym) = symtab.symbols.get(sym_idx) {
                    let size = sym.size as usize;
                    if size > 0 {
                        // Prefer resolved defining address
                        let src_vaddr = resolve_symbol(&sym.name).unwrap_or(sym.addr);
                        let mut remaining = size;
                        let mut offset = 0usize;
                        while remaining > 0 {
                            let chunk = core::cmp::min(8, remaining);
                            if let Some(src_val) = read_u64_at(src_vaddr + offset as u64) {
                                let bytes = &src_val.to_le_bytes()[0..chunk];
                                write_bytes_at(reloc_vaddr + offset as u64, bytes);
                            } else {
                                break;
                            }
                            remaining -= chunk;
                            offset += chunk;
                        }
                    }
                }
            }
            _ => {
                klog_warn!(
                    "Unhandled relocation type {} (in-place) at vaddr {:#x} (sym_idx={})",
                    reloc_type,
                    reloc_vaddr,
                    sym_idx
                );
            }
        }
    }
}
