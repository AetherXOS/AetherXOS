use alloc::string::ToString;

use xmas_elf::header::{Class, Data, Version};

pub(super) const DEFAULT_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES: usize = 2 * 1024 * 1024;
const MIN_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES: usize = 64 * 1024;
const MAX_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES: usize = 32 * 1024 * 1024;
const ELF64_EHDR_SIZE: usize = 64;
const ELF64_PHDR_MIN_SIZE: usize = 56;
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
pub(super) const ELF64_SYM_SIZE_BYTES: usize = 24;
pub(super) const MAX_HEURISTIC_SYMBOL_COUNT: usize = 1_000_000;

#[inline(always)]
pub(super) fn elf_machine_matches_target(machine: xmas_elf::header::Machine) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        machine == xmas_elf::header::Machine::X86_64
    }

    #[cfg(target_arch = "aarch64")]
    {
        machine == xmas_elf::header::Machine::AArch64
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let _ = machine;
        false
    }
}

pub(super) fn read_dynstr_entry(
    image: &[u8],
    strtab_off: usize,
    name_off: u64,
) -> Option<alloc::string::String> {
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
}

pub(super) fn split_search_paths(raw: &str) -> alloc::vec::Vec<alloc::string::String> {
    raw.split(':')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

#[inline]
fn read_u16_le(bytes: &[u8], off: usize) -> Option<u16> {
    let raw: [u8; 2] = bytes.get(off..off + 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(raw))
}

#[inline]
fn read_u32_le(bytes: &[u8], off: usize) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(off..off + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(raw))
}

#[inline]
fn read_u64_le(bytes: &[u8], off: usize) -> Option<u64> {
    let raw: [u8; 8] = bytes.get(off..off + 8)?.try_into().ok()?;
    Some(u64::from_le_bytes(raw))
}

pub(super) fn estimate_image_window_bytes(entry_addr: u64) -> Option<usize> {
    let ehdr = unsafe { core::slice::from_raw_parts(entry_addr as *const u8, ELF64_EHDR_SIZE) };
    if ehdr.get(0..4) != Some(b"\x7FELF") {
        return None;
    }
    if ehdr.get(4).copied() != Some(2) || ehdr.get(5).copied() != Some(1) {
        return None;
    }

    let phoff = read_u64_le(ehdr, 32)? as usize;
    let phentsize = read_u16_le(ehdr, 54)? as usize;
    let phnum = read_u16_le(ehdr, 56)? as usize;
    if phnum == 0 || phentsize < ELF64_PHDR_MIN_SIZE {
        return None;
    }

    let phdr_table_end = phoff.checked_add(phentsize.checked_mul(phnum)?)?;
    if phdr_table_end > MAX_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES {
        return None;
    }

    let phdr_span = unsafe { core::slice::from_raw_parts(entry_addr as *const u8, phdr_table_end) };
    let mut max_file_end = phdr_table_end;
    for i in 0..phnum {
        let base = phoff + i * phentsize;
        let p_type = read_u32_le(phdr_span, base)?;
        if p_type != PT_LOAD && p_type != PT_DYNAMIC {
            continue;
        }
        let p_offset = read_u64_le(phdr_span, base + 8)? as usize;
        let p_filesz = read_u64_le(phdr_span, base + 32)? as usize;
        let end = p_offset.checked_add(p_filesz)?;
        max_file_end = max_file_end.max(end);
    }

    let clamped = max_file_end
        .max(MIN_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES)
        .min(MAX_DYNAMIC_LINKER_IMAGE_WINDOW_BYTES);
    Some(clamped)
}

pub(super) fn is_supported_elf(elf: &xmas_elf::ElfFile<'_>) -> bool {
    elf.header.pt1.class() == Class::SixtyFour
        && elf.header.pt1.data() == Data::LittleEndian
        && elf.header.pt1.version() == Version::Current
        && elf_machine_matches_target(elf.header.pt2.machine().as_machine())
}
