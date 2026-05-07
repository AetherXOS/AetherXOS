use super::super::*;
use super::super::support::{ELF_HEADER_MIN_BYTES, elf_machine_matches_target};
use xmas_elf::ElfFile;
use xmas_elf::header::{Class, Data, Version};
use super::super::ModuleLoadError;
use super::super::support::checked_table_end;
use xmas_elf::program::Type;
use super::super::support::{current_target_elf_machine};
use super::super::ModuleInfo;
use super::super::{PARSE_ATTEMPTS, PARSE_FAILURES};
use core::sync::atomic::Ordering;

pub fn parse_elf(image: &[u8]) -> Result<ElfFile<'_>, ModuleLoadError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse begin\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse elf begin\n");
    crate::kernel::debug_trace::record_optional(
        "loader.parse",
        "align_mask",
        Some((image.as_ptr() as usize & 0xF) as u64),
        false,
    );
    if image.len() < ELF_HEADER_MIN_BYTES {
        return Err(ModuleLoadError::TooSmall);
    }

    let elf = ElfFile::new(image).map_err(|_| ModuleLoadError::ParseFailed)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse hdr\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse elf header returned\n");

    if elf.header.pt1.class() != Class::SixtyFour {
        return Err(ModuleLoadError::UnsupportedClass);
    }

    if elf.header.pt1.data() != Data::LittleEndian {
        return Err(ModuleLoadError::UnsupportedEndian);
    }

    if elf.header.pt1.version() != Version::Current {
        return Err(ModuleLoadError::UnsupportedVersion);
    }

    let machine = elf.header.pt2.machine().as_machine();
    if !elf_machine_matches_target(machine) {
        return Err(ModuleLoadError::UnsupportedMachine);
    }

    let phoff = elf.header.pt2.ph_offset() as usize;
    let shoff = elf.header.pt2.sh_offset() as usize;
    let phentsize = elf.header.pt2.ph_entry_size() as usize;
    let phnum = elf.header.pt2.ph_count() as usize;
    let shentsize = elf.header.pt2.sh_entry_size() as usize;
    let shnum = elf.header.pt2.sh_count() as usize;

    if phnum > 0 && checked_table_end(phoff, phnum, phentsize, image.len()).is_none() {
        return Err(ModuleLoadError::ProgramHeaderOutOfBounds);
    }

    if shnum > 0 && checked_table_end(shoff, shnum, shentsize, image.len()).is_none() {
        return Err(ModuleLoadError::SectionHeaderOutOfBounds);
    }
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse tbl\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse elf tables returned\n");

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse ok\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader parse elf returned\n");
    Ok(elf)
}

pub(crate) fn inspect_elf_from_parsed(elf: &ElfFile<'_>) -> ModuleInfo {
    let entry = elf.header.pt2.entry_point();
    let phnum = elf.header.pt2.ph_count() as usize;
    let phentsize = elf.header.pt2.ph_entry_size();
    let phaddr = elf
        .program_iter()
        .find(|ph| matches!(ph.get_type(), Ok(Type::Phdr)))
        .map(|ph| ph.virtual_addr())
        .or_else(|| {
            let phoff = elf.header.pt2.ph_offset();
            elf.program_iter().find_map(|ph| {
                if !matches!(ph.get_type(), Ok(Type::Load)) {
                    return None;
                }
                let seg_off = ph.offset();
                let seg_end = seg_off.checked_add(ph.file_size())?;
                if phoff >= seg_off && phoff < seg_end {
                    ph.virtual_addr().checked_add(phoff - seg_off)
                } else {
                    None
                }
            })
        })
        .unwrap_or(0);
    let shnum = elf.header.pt2.sh_count() as usize;

    let interpreter_path = elf
        .program_iter()
        .find(|ph| matches!(ph.get_type(), Ok(Type::Interp)))
        .and_then(|ph| {
            let off = ph.offset() as usize;
            let sz = ph.file_size() as usize;
            if off + sz <= elf.input.len() {
                let bytes = &elf.input[off..off + sz];
                // Strip null terminator if present
                let path_bytes = if bytes.last() == Some(&0) { &bytes[..sz-1] } else { bytes };
                core::str::from_utf8(path_bytes).ok().map(alloc::string::String::from)
            } else {
                None
            }
        });

    ModuleInfo {
        entry,
        program_headers: phnum as u16,
        program_header_entry_size: phentsize,
        program_header_addr: phaddr,
        section_headers: shnum as u16,
        machine: current_target_elf_machine(),
        interpreter_path,
    }
}


pub fn inspect_elf_image(image: &[u8]) -> Result<ModuleInfo, ModuleLoadError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader inspect image begin\n");
    PARSE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let elf = match parse_elf(image) {
        Ok(elf) => elf,
        Err(err) => {
            PARSE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };

    PARSE_SUCCESS.fetch_add(1, Ordering::Relaxed);

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader inspect image returned\n");
    Ok(inspect_elf_from_parsed(&elf))
}
