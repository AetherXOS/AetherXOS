//! Symbol resolution logic for dynamic linker

use alloc::string::ToString;

#[derive(Clone)]
pub struct Symbol {
    pub name: alloc::string::String,
    pub addr: u64,
    pub size: u64,
    pub st_info: u8,
    pub vers: Option<u16>,
    pub vers_name: Option<alloc::string::String>,
}

impl Symbol {
    pub fn binding(&self) -> u8 {
        self.st_info >> 4
    }

    pub fn is_undefined(&self) -> bool {
        self.addr == 0
    }
}
#[derive(Clone)]
pub struct SymbolTable {
    pub symbols: alloc::vec::Vec<Symbol>,
}

impl SymbolTable {
    pub fn parse(
        image: &[u8],
        strtab_offset: u64,
        symtab_offset: u64,
        count: usize,
        versym_file_offset: Option<u64>,
        verneed_file_offset: Option<u64>,
        verdef_file_offset: Option<u64>,
    ) -> Option<Self> {
        let mut symbols = alloc::vec::Vec::new();
        let strtab = &image[strtab_offset as usize..];
        let entry_size = 24; // ELF64 symbol entry
        for i in 0..count {
            let off = symtab_offset as usize + i * entry_size;
            if off + entry_size > image.len() {
                return None;
            }
            let name_off = u32::from_le_bytes(image[off..off + 4].try_into().ok()?);
            let addr = u64::from_le_bytes(image[off + 8..off + 16].try_into().ok()?);
            let size = u64::from_le_bytes(image[off + 16..off + 24].try_into().ok()?);
            // Read symbol name from strtab
            let mut end = name_off as usize;
            while end < strtab.len() && strtab[end] != 0 {
                end += 1;
            }
            let st_info = image[off + 4];
            let name = core::str::from_utf8(&strtab[name_off as usize..end])
                .ok()?
                .to_string();
            symbols.push(Symbol {
                name,
                addr,
                size,
                st_info,
                vers: None,
                vers_name: None,
            });
        }

        // If versym table provided, read u16 entries and assign to symbols
        if let Some(voff) = versym_file_offset {
            let vbase = voff as usize;
            // Ensure there's enough data for count u16 entries
            if vbase + count * 2 <= image.len() {
                for i in 0..count {
                    let idx = vbase + i * 2;
                    let ver = u16::from_le_bytes(image[idx..idx + 2].try_into().ok()?);
                    if let Some(s) = symbols.get_mut(i) {
                        s.vers = Some(ver);
                    }
                }
            }
        }

        // If verneed table provided, parse version names and assign them where vers index matches.
        if let Some(vn_off) = verneed_file_offset {
            let mut idx = vn_off as usize;
            while idx + 16 <= image.len() {
                // Elf64_Verneed: vn_version u16, vn_cnt u16, vn_file u32, vn_aux u32, vn_next u32
                let _vn_version = u16::from_le_bytes(image[idx..idx + 2].try_into().ok()?);
                let vn_cnt = u16::from_le_bytes(image[idx + 2..idx + 4].try_into().ok()?);
                let _vn_file = u32::from_le_bytes(image[idx + 4..idx + 8].try_into().ok()?);
                let vn_aux = u32::from_le_bytes(image[idx + 8..idx + 12].try_into().ok()?);
                let vn_next = u32::from_le_bytes(image[idx + 12..idx + 16].try_into().ok()?);

                // Walk aux entries
                let mut aidx = idx + vn_aux as usize;
                for _ in 0..vn_cnt {
                    if aidx + 16 > image.len() {
                        break;
                    }
                    // Elf64_Vernaux: vna_hash u32, vna_other u16, vna_name u32, vna_next u32 (layout varies)
                    let _vna_hash = u32::from_le_bytes(image[aidx..aidx + 4].try_into().ok()?);
                    let vna_other = u16::from_le_bytes(image[aidx + 4..aidx + 6].try_into().ok()?);
                    let vna_name = u32::from_le_bytes(image[aidx + 8..aidx + 12].try_into().ok()?);
                    // vna_name is offset into dynstr (strtab)
                    let mut end = vna_name as usize;
                    while end < strtab.len() && strtab[end] != 0 {
                        end += 1;
                    }
                    if let Some(name) = core::str::from_utf8(&strtab[vna_name as usize..end]).ok() {
                        // assign version name to any symbol that has vers == vna_other
                        for s in symbols.iter_mut() {
                            if s.vers == Some(vna_other) {
                                s.vers_name = Some(name.to_string());
                            }
                        }
                    }
                    let vna_next = u32::from_le_bytes(image[aidx + 12..aidx + 16].try_into().ok()?);
                    if vna_next == 0 {
                        break;
                    }
                    aidx += vna_next as usize;
                }

                if vn_next == 0 {
                    break;
                }
                idx += vn_next as usize;
            }
        }

        // If verdef table provided, parse version definitions and assign names for version indices.
        if let Some(vd_off) = verdef_file_offset {
            let mut idx = vd_off as usize;
            while idx + 20 <= image.len() {
                // Elf64_Verdef: vd_version u16, vd_flags u16, vd_ndx u16, vd_cnt u16, vd_hash u32, vd_aux u32, vd_next u32
                let _vd_version = u16::from_le_bytes(image[idx..idx + 2].try_into().ok()?);
                let _vd_flags = u16::from_le_bytes(image[idx + 2..idx + 4].try_into().ok()?);
                let vd_ndx = u16::from_le_bytes(image[idx + 4..idx + 6].try_into().ok()?);
                let _vd_cnt = u16::from_le_bytes(image[idx + 6..idx + 8].try_into().ok()?);
                let _vd_hash = u32::from_le_bytes(image[idx + 8..idx + 12].try_into().ok()?);
                let vd_aux = u32::from_le_bytes(image[idx + 12..idx + 16].try_into().ok()?);
                let vd_next = u32::from_le_bytes(image[idx + 16..idx + 20].try_into().ok()?);

                // Walk aux entries to get name
                let aidx = idx + vd_aux as usize;
                if aidx + 8 <= image.len() {
                    // Elf64_Verdaux: vda_name u32, vda_next u32
                    let vda_name = u32::from_le_bytes(image[aidx..aidx + 4].try_into().ok()?);
                    // name offset into strtab
                    let mut end = vda_name as usize;
                    while end < strtab.len() && strtab[end] != 0 {
                        end += 1;
                    }
                    if let Some(name) = core::str::from_utf8(&strtab[vda_name as usize..end]).ok() {
                        // assign to symbols matching this version index
                        for s in symbols.iter_mut() {
                            if s.vers == Some(vd_ndx) {
                                s.vers_name = Some(name.to_string());
                            }
                        }
                    }
                }

                if vd_next == 0 {
                    break;
                }
                idx += vd_next as usize;
            }
        }
        Some(SymbolTable { symbols })
    }

    pub fn is_ifunc(sym: &Symbol) -> bool {
        // ST_TYPE is low 4 bits of st_info. STT_GNU_IFUNC is 10.
        (sym.st_info & 0xF) == 10
    }

    /// Advanced symbol resolution: global/local, weak/strong, preemption, versioning
    pub fn resolve<'a>(
        &'a self,
        name: &str,
        global_scope: &[&'a SymbolTable],
        version: Option<&str>,
    ) -> Option<&'a Symbol> {
        const STB_WEAK: u8 = 2;

        // Helper to match name+version predicate
        let matches = |s: &Symbol| -> bool {
            s.name == name && (version.is_none() || s.vers_name.as_deref() == version)
        };

        // 1) Prefer a local strong, defined symbol
        if let Some(local_strong) = self
            .symbols
            .iter()
            .find(|s| matches(s) && !s.is_undefined() && s.binding() != STB_WEAK)
        {
            return Some(local_strong);
        }

        // 2) If local has a defined weak, it can be preempted by a global strong.
        let local_weak_defined = self
            .symbols
            .iter()
            .find(|s| matches(s) && !s.is_undefined() && s.binding() == STB_WEAK);

        // 3) Search global scope for a strong defined symbol (can preempt local weak)
        for table in global_scope {
            if let Some(gstrong) = table
                .symbols
                .iter()
                .find(|s| matches(s) && !s.is_undefined() && s.binding() != STB_WEAK)
            {
                return Some(gstrong);
            }
        }

        // 4) If no global strong, prefer local weak defined symbol if present
        if let Some(lw) = local_weak_defined {
            return Some(lw);
        }

        // 5) Otherwise take any global match (strong or weak), even if undefined (last resort)
        for table in global_scope {
            if let Some(g) = table.symbols.iter().find(|s| matches(s)) {
                return Some(g);
            }
        }

        // 6) Finally, if nothing found, try any local match (including undefined)
        self.symbols.iter().find(|s| matches(s))
    }

    /// Simple name lookup convenience wrapper used by callers that don't need global scope/versioning.
    pub fn find_by_name(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }
}
