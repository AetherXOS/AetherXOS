use super::*;
use crate::kernel::symbol::{Symbol, SymbolTable};

#[path = "early_part1.rs"]
mod early_part1;

#[test]
fn test_plt32_image_signed_wrap() {
    let mut image = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x0;
    let sym_addr: u64 = 0x8000_0000u64 + 7;
    let addend: i64 = 0x10;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("psym"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::PLT32 as u64),
        addend: Some(addend as u64),
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
    let expected = ((sym_addr as i64 + addend as i64) - place_vaddr as i64) as i32;
    assert_eq!(written, expected);
}

#[test]
fn test_plt32_inplace_signed_wrap() {
    let mut mem = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x20;
    let sym_addr: u64 = 0x9000_0000u64 + 0x20;
    let addend: i64 = 0x40;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("psym2"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::PLT32 as u64),
        addend: Some(addend as u64),
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
    let expected = ((sym_addr as i64 + addend as i64) - place_vaddr as i64) as i32;
    assert_eq!(written, expected);
}

#[test]
fn test_tlsgd_image_writes_u64() {
    // Image relocation for TLSGD/TLSLD should write S + A when symbol present
    let mut image = vec![0u8; 0x2000];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x800;
    let sym_addr: u64 = 0x1800;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("tls_sym"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::TLSGD as u64),
        addend: Some(0x10),
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
    let expected = sym_addr.wrapping_add(0x10);
    assert_eq!(written, expected);
}

#[test]
fn test_tlsgd_inplace_writes_u64() {
    // In-place relocation should write S + A when symbol present
    let mut mem = vec![0u8; 0x3000];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x900;
    let sym_addr: u64 = 0x1_0000;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("tls_sym2"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::TLSGD as u64),
        addend: Some(0x20),
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
    let expected = sym_addr.wrapping_add(0x20);
    assert_eq!(written, expected);
}

#[test]
fn test_irelative_image_writes_b_plus_addend() {
    // Image relocation for IRELATIVE should write B + A when resolver not available
    let mut image = vec![0u8; 0x200];
    let base: u64 = 0x4000;
    let place_vaddr: u64 = 0x80;
    let addend: u64 = 0x200;

    let rel = RelocationEntry {
        offset: place_vaddr,
        info: (0u64 << 32) | (RelocTypeX86_64::IRELATIVE as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    // empty symbol table, no resolver
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::new(),
    };
    let v2o = |_v: u64| -> Option<usize> { Some(_v as usize) };
    let resolve = |_n: &str| -> Option<u64> { None };

    process_relocations(&rel_table, &mut image, base, &symtab, &resolve, &v2o);

    let written = u64::from_le_bytes(
        image[place_vaddr as usize..place_vaddr as usize + 8]
            .try_into()
            .unwrap(),
    );
    assert_eq!(written, base.wrapping_add(addend));
}

