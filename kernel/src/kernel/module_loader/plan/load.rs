use super::super::*;
use super::elf::{inspect_elf_from_parsed, parse_elf};
use xmas_elf::ElfFile;
use super::super::{ModuleLoadError, ModuleLoadPlan, LoadSegmentPlan, ModuleInfo};
use xmas_elf::program::Type;
use super::super::support::{segment_range_fits_image, entry_in_segments};
use core::sync::atomic::Ordering;

pub(crate) fn build_load_plan_from_parsed(
    elf: &ElfFile<'_>,
    image_len: usize,
    info: &ModuleInfo,
) -> Result<ModuleLoadPlan, ModuleLoadError> {
    let mut segments: Vec<LoadSegmentPlan> = Vec::new();
    let mut total_file_bytes = 0u64;
    let mut total_mem_bytes = 0u64;
    let mut tls_virtual_addr = 0u64;
    let mut tls_file_size = 0u64;
    let mut tls_mem_size = 0u64;
    let mut tls_align = 0u64;

    let aslr_base = {
        let tsc = crate::hal::cpu::rdtsc();
        let mask: u64 = 0x7fff_0000;
        crate::kernel::syscalls::syscalls_consts::linux::BRK_START as u64 + (tsc & mask)
    };
    let enforce_segment_congruence = crate::config::KernelConfig::exec_elf_enforce_segment_congruence();

    crate::kernel::debug_trace::record_optional(
        "loader.plan",
        "iter_begin",
        Some(elf.header.pt2.ph_count() as u64),
        false,
    );
    for ph in elf.program_iter() {
        if matches!(ph.get_type(), Ok(Type::Tls)) {
            tls_virtual_addr = ph.virtual_addr() + aslr_base;
            tls_file_size = ph.file_size();
            tls_mem_size = ph.mem_size();
            tls_align = ph.align();
        }
        if !matches!(ph.get_type(), Ok(Type::Load)) {
            continue;
        }

        if segments.len() >= crate::config::KernelConfig::module_loader_max_load_segments() {
            return Err(ModuleLoadError::TooManyLoadSegments);
        }

        let file_offset = ph.offset();
        let file_size = ph.file_size();
        let mem_size = ph.mem_size();
        let virtual_addr = ph.virtual_addr();
        let align = ph.align();

        if enforce_segment_congruence && align > 1 {
            if (virtual_addr % align) != (file_offset % align) {
                return Err(ModuleLoadError::SegmentAlignmentMismatch);
            }
        }

        if file_size > mem_size {
            return Err(ModuleLoadError::SegmentFileExceedsMem);
        }

        if !segment_range_fits_image(file_offset, file_size, image_len) {
            return Err(ModuleLoadError::SegmentOutOfBounds);
        }

        total_file_bytes = total_file_bytes.saturating_add(file_size);
        total_mem_bytes = total_mem_bytes.saturating_add(mem_size);

        if total_mem_bytes > crate::config::KernelConfig::module_loader_max_total_image_bytes() {
            return Err(ModuleLoadError::ImageTooLarge);
        }

        let segment_virtual_addr = virtual_addr
            .checked_add(aslr_base)
            .ok_or(ModuleLoadError::SegmentAddressOverflow)?;
        let segment_virtual_end = segment_virtual_addr
            .checked_add(mem_size)
            .ok_or(ModuleLoadError::SegmentAddressOverflow)?;

        for existing in &segments {
            let existing_end = existing
                .virtual_addr
                .checked_add(existing.mem_size)
                .ok_or(ModuleLoadError::SegmentAddressOverflow)?;
            if segment_virtual_addr < existing_end && existing.virtual_addr < segment_virtual_end {
                return Err(ModuleLoadError::SegmentOverlap);
            }
        }

        segments.push(LoadSegmentPlan {
            virtual_addr: segment_virtual_addr,
            file_offset,
            file_size,
            mem_size,
            align,
        });
    }
    crate::kernel::debug_trace::record_optional(
        "loader.plan",
        "iter_returned",
        Some(segments.len() as u64),
        false,
    );

    if segments.is_empty() {
        return Err(ModuleLoadError::NoLoadSegments);
    }

    let entry = elf.header.pt2.entry_point() + aslr_base;
    let segment_ranges: Vec<(u64, u64)> = segments
        .iter()
        .map(|segment| (segment.virtual_addr, segment.mem_size))
        .collect();
    if !entry_in_segments(entry, &segment_ranges) {
        return Err(ModuleLoadError::EntryOutsideLoadSegments);
    }

    let phnum = elf.header.pt2.ph_count() as usize;
    let phentsize = elf.header.pt2.ph_entry_size();
    let phaddr = info.program_header_addr;

    Ok(ModuleLoadPlan {
        entry,
        segments,
        total_file_bytes,
        total_mem_bytes,
        aslr_base,
        tls_virtual_addr,
        tls_file_size,
        tls_mem_size,
        tls_align,
        program_header_addr: phaddr
            .checked_add(aslr_base)
            .ok_or(ModuleLoadError::SegmentAddressOverflow)?,
        program_header_entry_size: phentsize,
        program_headers: phnum as u16,
    })
}

pub fn build_load_plan(image: &[u8]) -> Result<ModuleLoadPlan, ModuleLoadError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader build load plan body begin\n");
    PLAN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let elf = match parse_elf(image) {
        Ok(elf) => elf,
        Err(err) => {
            PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };

    let info = inspect_elf_from_parsed(&elf);
    let plan = build_load_plan_from_parsed(&elf, image.len(), &info).inspect_err(|_| {
        PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
    })?;

    PLAN_SUCCESS.fetch_add(1, Ordering::Relaxed);

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader build load plan body returned\n");
    Ok(plan)
}
