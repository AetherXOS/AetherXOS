use alloc::string::String;

pub struct DynamicEntry {
    pub tag: u64,
    pub val: u64,
}

pub struct DynamicSection {
    pub entries: alloc::vec::Vec<DynamicEntry>,
    // Indexed DT_* values for fast lookup
    pub dt_needed: alloc::vec::Vec<u64>,
    pub dt_rpath: Option<u64>,
    pub dt_runpath: Option<u64>,
    pub dt_hash: Option<u64>,
    pub dt_gnu_hash: Option<u64>,
    pub dt_symtab: Option<u64>,
    pub dt_strtab: Option<u64>,
    pub dt_strsz: Option<u64>,
    pub dt_syment: Option<u64>,
    pub dt_versym: Option<u64>,
    pub dt_verdef: Option<u64>,
    pub dt_verdefnum: Option<u64>,
    pub dt_verneed: Option<u64>,
    pub dt_verneednum: Option<u64>,
    pub dt_init: Option<u64>,
    pub dt_fini: Option<u64>,
    pub dt_pltgot: Option<u64>,
    pub dt_jmprel: Option<u64>,
    pub dt_pltrel: Option<u64>,
    pub dt_pltrelsz: Option<u64>,
    pub dt_rel: Option<u64>,
    pub dt_relsz: Option<u64>,
    pub dt_relent: Option<u64>,
    pub dt_rela: Option<u64>,
    pub dt_relasz: Option<u64>,
    pub dt_relaent: Option<u64>,
    pub dt_init_array: Option<u64>,
    pub dt_init_arraysz: Option<u64>,
    pub dt_fini_array: Option<u64>,
    pub dt_fini_arraysz: Option<u64>,
    pub dt_preinit_array: Option<u64>,
    pub dt_preinit_arraysz: Option<u64>,
    pub dt_soname: Option<u64>,
    pub dt_flags: Option<u64>,
    pub dt_flags_1: Option<u64>,
    pub dt_debug: Option<u64>,
    pub dt_textrel: Option<u64>,
    pub dt_bind_now: Option<u64>,
    pub dt_runpath_str: Option<String>,
    pub dt_rpath_str: Option<String>,
    pub dt_soname_str: Option<String>,
}

impl DynamicSection {
    pub fn parse(image: &[u8], dynamic_offset: u64, count: usize) -> Option<Self> {
        let mut entries = alloc::vec::Vec::new();
        let mut dt_needed = alloc::vec::Vec::new();
        let mut dt_rpath = None;
        let mut dt_runpath = None;
        let mut dt_hash = None;
        let mut dt_gnu_hash = None;
        let mut dt_symtab = None;
        let mut dt_strtab = None;
        let mut dt_strsz = None;
        let mut dt_syment = None;
        let mut dt_versym = None;
        let mut dt_verdef = None;
        let mut dt_verdefnum = None;
        let mut dt_verneed = None;
        let mut dt_verneednum = None;
        let mut dt_init = None;
        let mut dt_fini = None;
        let mut dt_pltgot = None;
        let mut dt_jmprel = None;
        let mut dt_pltrel = None;
        let mut dt_pltrelsz = None;
        let mut dt_rel = None;
        let mut dt_relsz = None;
        let mut dt_relent = None;
        let mut dt_rela = None;
        let mut dt_relasz = None;
        let mut dt_relaent = None;
        let mut dt_init_array = None;
        let mut dt_init_arraysz = None;
        let mut dt_fini_array = None;
        let mut dt_fini_arraysz = None;
        let mut dt_preinit_array = None;
        let mut dt_preinit_arraysz = None;
        let mut dt_soname = None;
        let mut dt_flags = None;
        let mut dt_flags_1 = None;
        let mut dt_debug = None;
        let mut dt_textrel = None;
        let mut dt_bind_now = None;
        let dt_runpath_str = None;
        let dt_rpath_str = None;
        let dt_soname_str = None;
        let mut _dt_symbolic: Option<u64> = None;
        let base = dynamic_offset as usize;
        let entry_size = 16; // Each entry is 2 x u64
        for i in 0..count {
            let off = base + i * entry_size;
            if off + entry_size > image.len() {
                return None;
            }
            let tag = u64::from_le_bytes(image[off..off + 8].try_into().ok()?);
            let val = u64::from_le_bytes(image[off + 8..off + 16].try_into().ok()?);
            entries.push(DynamicEntry { tag, val });
            match tag {
                1 => dt_needed.push(val), // DT_NEEDED
                2 => dt_pltrelsz = Some(val),
                3 => dt_pltgot = Some(val),
                4 => dt_hash = Some(val),
                5 => dt_strtab = Some(val),
                6 => dt_symtab = Some(val),
                7 => dt_rela = Some(val),
                8 => dt_relasz = Some(val),
                9 => dt_relaent = Some(val),
                10 => dt_strsz = Some(val),
                11 => dt_syment = Some(val),
                12 => dt_init = Some(val),
                13 => dt_fini = Some(val),
                14 => dt_soname = Some(val),
                15 => dt_rpath = Some(val),
                16 => _dt_symbolic = Some(val), // DT_SYMBOLIC (deprecated)
                17 => dt_rel = Some(val),
                18 => dt_relsz = Some(val),
                19 => dt_relent = Some(val),
                20 => dt_pltrel = Some(val),
                21 => dt_debug = Some(val),
                22 => dt_textrel = Some(val),
                23 => dt_jmprel = Some(val),
                24 => dt_bind_now = Some(val),
                25 => dt_init_array = Some(val),
                26 => dt_init_arraysz = Some(val),
                27 => dt_fini_array = Some(val),
                28 => dt_fini_arraysz = Some(val),
                29 => dt_runpath = Some(val),
                30 => dt_flags = Some(val),
                32 => dt_preinit_array = Some(val),
                33 => dt_preinit_arraysz = Some(val),
                0x6ffffffb => dt_flags_1 = Some(val),
                0x6ffffef5 => dt_gnu_hash = Some(val),
                0x6ffffff0 => dt_versym = Some(val),
                0x6ffffffc => dt_verdef = Some(val),
                0x6ffffffd => dt_verdefnum = Some(val),
                0x6ffffffe => dt_verneed = Some(val),
                0x6fffffff => dt_verneednum = Some(val),
                _ => {}
            }
        }
        Some(DynamicSection {
            entries,
            dt_needed,
            dt_rpath,
            dt_runpath,
            dt_hash,
            dt_gnu_hash,
            dt_symtab,
            dt_strtab,
            dt_strsz,
            dt_syment,
            dt_versym,
            dt_verdef,
            dt_verdefnum,
            dt_verneed,
            dt_verneednum,
            dt_init,
            dt_fini,
            dt_pltgot,
            dt_jmprel,
            dt_pltrel,
            dt_pltrelsz,
            dt_rel,
            dt_relsz,
            dt_relent,
            dt_rela,
            dt_relasz,
            dt_relaent,
            dt_init_array,
            dt_init_arraysz,
            dt_fini_array,
            dt_fini_arraysz,
            dt_preinit_array,
            dt_preinit_arraysz,
            dt_soname,
            dt_flags,
            dt_flags_1,
            dt_debug,
            dt_textrel,
            dt_bind_now,
            dt_runpath_str,
            dt_rpath_str,
            dt_soname_str,
        })
    }

    pub fn get_dt_value(&self, tag: u64) -> Option<u64> {
        self.entries.iter().find(|e| e.tag == tag).map(|e| e.val)
    }
}

pub struct RelocationEntry {
    pub offset: u64,
    pub info: u64,
    pub addend: Option<u64>,
}

