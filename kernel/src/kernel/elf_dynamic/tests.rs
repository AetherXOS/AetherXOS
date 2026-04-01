use super::*;
use crate::kernel::symbol::{Symbol, SymbolTable};

#[path = "tests/early.rs"]
mod early;

#[test_case]
fn test_tpoff32_image_writes_s32() {
    let mut image = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x40;
    let sym_addr: u64 = 0x1000;
    let addend: u64 = 0x10;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("tpsym"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::TPOFF32 as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let v2o = |_v: u64| -> Option<usize> { Some(_v as usize) };
    let resolve = |_n: &str| -> Option<u64> { Some(sym_addr) };

    process_relocations(&rel_table, &mut image, base, &symtab, &resolve, &v2o);

    let written = i32::from_le_bytes(
        image[place_vaddr as usize..place_vaddr as usize + 4]
            .try_into()
            .unwrap(),
    );
    let expected = (sym_addr as i64 + addend as i64) as i32;
    assert_eq!(written, expected);
}

#[test_case]
fn test_tpoff32_inplace_writes_s32() {
    let mut mem = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x48;
    let sym_addr: u64 = 0x2000;
    let addend: u64 = 0x20;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("tpsym2"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::TPOFF32 as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let read_u64_at = |_v: u64| -> Option<u64> { None };
    let read_u32_at = |_v: u64| -> Option<u32> { None };
    let write_u64_at = |_v: u64, _val: u64| {};
    let write_bytes_at = |v: u64, b: &[u8]| {
        let off = v as usize;
        mem[off..off + b.len()].copy_from_slice(b);
    };

    process_relocations_inplace(
        &rel_table,
        base,
        &symtab,
        &|_n| Some(sym_addr),
        &read_u64_at,
        &read_u32_at,
        &write_u64_at,
        &write_bytes_at,
    );

    let written = i32::from_le_bytes(
        mem[place_vaddr as usize..place_vaddr as usize + 4]
            .try_into()
            .unwrap(),
    );
    let expected = (sym_addr as i64 + addend as i64) as i32;
    assert_eq!(written, expected);
}

#[test_case]
fn test_tpoff64_image_writes_u64() {
    let mut image = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x50;
    let sym_addr: u64 = 0x3000;
    let addend: u64 = 0x30;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("tpsym64"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::TPOFF64 as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let v2o = |_v: u64| -> Option<usize> { Some(_v as usize) };
    let resolve = |_n: &str| -> Option<u64> { Some(sym_addr) };

    process_relocations(&rel_table, &mut image, base, &symtab, &resolve, &v2o);

    let written = u64::from_le_bytes(
        image[place_vaddr as usize..place_vaddr as usize + 8]
            .try_into()
            .unwrap(),
    );
    let expected = (sym_addr as u64).wrapping_add(addend);
    assert_eq!(written, expected);
}

#[test_case]
fn test_tpoff64_inplace_writes_u64() {
    let mut mem = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x60;
    let sym_addr: u64 = 0x4000;
    let addend: u64 = 0x40;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("tpsym64b"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::TPOFF64 as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let read_u64_at = |_v: u64| -> Option<u64> { None };
    let read_u32_at = |_v: u64| -> Option<u32> { None };
    let write_u64_at = |v: u64, val: u64| {
        let off = v as usize;
        mem[off..off + 8].copy_from_slice(&val.to_le_bytes());
    };
    let write_bytes_at = |_v: u64, _b: &[u8]| {};

    process_relocations_inplace(
        &rel_table,
        base,
        &symtab,
        &|_n| Some(sym_addr),
        &read_u64_at,
        &read_u32_at,
        &write_u64_at,
        &write_bytes_at,
    );

    let written = u64::from_le_bytes(
        mem[place_vaddr as usize..place_vaddr as usize + 8]
            .try_into()
            .unwrap(),
    );
    let expected = sym_addr.wrapping_add(addend);
    assert_eq!(written, expected);
}

#[test_case]
fn test_globdat_image_writes_u64_with_addend() {
    let mut image = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x80;
    let sym_addr: u64 = 0x1000;
    let addend: u64 = 0x200;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("gdsym"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::GLOB_DAT as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let v2o = |_v: u64| -> Option<usize> { Some(_v as usize) };
    let resolve = |_n: &str| -> Option<u64> { Some(sym_addr) };

    process_relocations(&rel_table, &mut image, base, &symtab, &resolve, &v2o);

    let written = u64::from_le_bytes(
        image[place_vaddr as usize..place_vaddr as usize + 8]
            .try_into()
            .unwrap(),
    );
    assert_eq!(written, sym_addr.wrapping_add(addend));
}

#[test_case]
fn test_globdat_inplace_writes_u64_with_addend() {
    let mut mem = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x90;
    let sym_addr: u64 = 0x2000;
    let addend: u64 = 0x300;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("gdsym2"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::GLOB_DAT as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let read_u64_at = |_v: u64| -> Option<u64> { None };
    let read_u32_at = |_v: u64| -> Option<u32> { None };
    let write_u64_at = |v: u64, val: u64| {
        let off = v as usize;
        mem[off..off + 8].copy_from_slice(&val.to_le_bytes());
    };
    let write_bytes_at = |_v: u64, _b: &[u8]| {};

    process_relocations_inplace(
        &rel_table,
        base,
        &symtab,
        &|_n| Some(sym_addr),
        &read_u64_at,
        &read_u32_at,
        &write_u64_at,
        &write_bytes_at,
    );

    let written = u64::from_le_bytes(
        mem[place_vaddr as usize..place_vaddr as usize + 8]
            .try_into()
            .unwrap(),
    );
    assert_eq!(written, sym_addr.wrapping_add(addend));
}

#[test_case]
fn test_got32_image_writes_u32() {
    let mut image = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x80;
    let sym_addr: u64 = 0x1234_5678;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("got32sym"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::GOT32 as u64),
        addend: Some(0),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let v2o = |_v: u64| -> Option<usize> { Some(_v as usize) };
    let resolve = |_n: &str| -> Option<u64> { Some(sym_addr) };

    process_relocations(&rel_table, &mut image, base, &symtab, &resolve, &v2o);

    let written = u32::from_le_bytes(
        image[place_vaddr as usize..place_vaddr as usize + 4]
            .try_into()
            .unwrap(),
    );
    assert_eq!(written as u64, sym_addr);
}

#[test_case]
fn test_got32_inplace_writes_u32() {
    let mut mem = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x90;
    let sym_addr: u64 = 0x8765_4321;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("got32sym2"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::GOT32 as u64),
        addend: Some(0),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let read_u64_at = |_v: u64| -> Option<u64> { None };
    let read_u32_at = |_v: u64| -> Option<u32> { None };
    let write_u64_at = |_v: u64, _val: u64| {};
    let write_bytes_at = |v: u64, b: &[u8]| {
        let off = v as usize;
        mem[off..off + b.len()].copy_from_slice(b);
    };

    process_relocations_inplace(
        &rel_table,
        base,
        &symtab,
        &|_n| Some(sym_addr),
        &read_u64_at,
        &read_u32_at,
        &write_u64_at,
        &write_bytes_at,
    );

    let written = u32::from_le_bytes(
        mem[place_vaddr as usize..place_vaddr as usize + 4]
            .try_into()
            .unwrap(),
    );
    assert_eq!(written as u64, sym_addr);
}

