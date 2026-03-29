use super::*;
use crate::kernel::symbol::{Symbol, SymbolTable};

#[test]
fn test_copy_relocation_image() {
    // Create image with source data at offset 0x100 and target at 0x200
    let mut image = vec![0u8; 0x400];
    let src = b"HELLO!!!";
    let src_off = 0x100usize;
    image[src_off..src_off + src.len()].copy_from_slice(src);

    // Symbol points to src_vaddr (we use addr == offset since base=0)
    let sym = Symbol {
        name: alloc::string::String::from("sym0"),
        addr: src_off as u64,
        size: src.len() as u64,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    // Build a RelocationEntry for COPY: offset = target_vaddr, info = (sym_idx<<32)|TYPE
    let target_vaddr = 0x200u64;
    let info = ((0usize as u64) << 32) | (RelocTypeX86_64::COPY as u64);
    let rel = RelocationEntry {
        offset: target_vaddr,
        info,
        addend: None,
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rel,
    };

    // vaddr_to_off maps identity for this test
    let v2o = |_v: u64| -> Option<usize> { Some(_v as usize) };
    let resolve = |_name: &str| -> Option<u64> { None };

    process_relocations(&rel_table, &mut image, 0, &symtab, &resolve, &v2o);

    assert_eq!(
        &image[target_vaddr as usize..target_vaddr as usize + src.len()],
        src
    );
}

#[test]
fn test_plt32_image_writes_rel32() {
    // Build a small image buffer and run process_relocations on it
    let mut image = vec![0u8; 0x3000];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x1000;
    let sym_addr: u64 = 0x2000;

    // Put initial 0 at place (for REL case would be read), but we use RELA
    // Symbol table: one symbol at sym_addr
    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("foo_img"),
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
        addend: Some(0),
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
    let expected = (sym_addr as i64 - place_vaddr as i64) as i32;
    assert_eq!(written, expected);
}

#[test]
fn test_gotpcrelx_image_writes_rel32() {
    let mut image = vec![0u8; 0x6000];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x3000;
    let sym_addr: u64 = 0x7000;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("baz_img"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::GOTPCRELX as u64),
        addend: Some(12),
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
    let expected = ((sym_addr as i64 + 12) - place_vaddr as i64) as i32;
    assert_eq!(written, expected);
}

#[test]
fn test_irelative_inplace_writes_base_plus_addend() {
    // In-place relocation should write B + A when symbol not present
    let mut mem = vec![0u8; 0x300];
    let base: u64 = 0x1000;
    let reloc_offset = 0x50u64;
    let addend: u64 = 0x1234;

    let rel = RelocationEntry {
        offset: reloc_offset,
        info: (0u64 << 32) | (RelocTypeX86_64::IRELATIVE as u64),
        addend: Some(addend),
    };
    let rel_table = RelocationTable {
        entries: alloc::vec::Vec::from([rel]),
        rel_type: RelocationType::Rela,
    };

    let symtab = SymbolTable {
        symbols: alloc::vec::Vec::new(),
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
        &|_n| None,
        &read_u64_at,
        &read_u32_at,
        &write_u64_at,
        &write_bytes_at,
    );

    let written = u64::from_le_bytes(
        mem[reloc_offset as usize..reloc_offset as usize + 8]
            .try_into()
            .unwrap(),
    );
    assert_eq!(written, base.wrapping_add(addend));
}

#[test]
fn test_plt32_inplace_writes_rel32() {
    // Prepare memory: symbol at 0x2000, place at 0x1000
    let mut mem = vec![0u8; 0x3000];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x1000;
    let sym_addr: u64 = 0x2000;

    // Build symbol table with one symbol at sym_addr
    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("foo"),
        addr: sym_addr,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = crate::kernel::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym]),
    };

    // Relocation: PLT32 with sym_idx=0 at offset=place_vaddr
    let rel = RelocationEntry {
        offset: place_vaddr,
        info: ((0u64) << 32) | (RelocTypeX86_64::PLT32 as u64),
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
        &|n| {
            if n == "foo" {
                Some(sym_addr)
            } else {
                None
            }
        },
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
    let expected = (sym_addr as i64 - place_vaddr as i64) as i32;
    assert_eq!(written, expected);
}

#[test]
fn test_gotpcrel_inplace_writes_rel32() {
    // GOTPCREL: place at 0x3000, sym at 0x4000, addend 4
    let mut mem = vec![0u8; 0x5000];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x3000;
    let sym_addr: u64 = 0x4000;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("bar"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::GOTPCREL as u64),
        addend: Some(4),
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
        &|n| {
            if n == "bar" {
                Some(sym_addr)
            } else {
                None
            }
        },
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
    let expected = ((sym_addr as i64 + 4) - place_vaddr as i64) as i32;
    assert_eq!(written, expected);
}

#[test]
fn test_gotpcrelx_inplace_writes_rel32() {
    // GOTPCRELX variant: behave like GOTPCREL for best-effort.
    let mut mem = vec![0u8; 0x5000];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x3000;
    let sym_addr: u64 = 0x5000;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("baz"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::GOTPCRELX as u64),
        addend: Some(8),
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
        &|n| {
            if n == "baz" {
                Some(sym_addr)
            } else {
                None
            }
        },
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
    let expected = ((sym_addr as i64 + 8) - place_vaddr as i64) as i32;
    assert_eq!(written, expected);
}

#[test]
fn test_gotpcrelx_image_signed_wrap() {
    // Make a value that overflows signed 32-bit when computing S + A - P
    let mut image = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x0;
    let sym_addr: u64 = 0x8000_0000u64 + 123; // > i32::MAX
    let addend: i64 = 0x200;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("wrap"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::GOTPCRELX as u64),
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
fn test_gotpcrelx_inplace_signed_wrap() {
    let mut mem = vec![0u8; 0x200];
    let base: u64 = 0;
    let place_vaddr: u64 = 0x10;
    let sym_addr: u64 = 0x9000_0000u64 + 500;
    let addend: i64 = 0x300;

    let sym = crate::kernel::symbol::Symbol {
        name: alloc::string::String::from("wrap2"),
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
        info: ((0u64) << 32) | (RelocTypeX86_64::GOTPCRELX as u64),
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

