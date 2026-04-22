use super::*;

fn parse_elf(image: &[u8]) -> Result<ElfFile<'_>, ModuleLoadError> {
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

fn inspect_elf_from_parsed(elf: &ElfFile<'_>) -> ModuleImageInfo {
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

    ModuleImageInfo {
        entry,
        program_headers: phnum as u16,
        program_header_entry_size: phentsize,
        program_header_addr: phaddr,
        section_headers: shnum as u16,
        machine: current_target_elf_machine(),
    }
}

fn build_load_plan_from_parsed(
    elf: &ElfFile<'_>,
    image_len: usize,
    info: &ModuleImageInfo,
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

fn build_virtual_mapping_plan_from_load_plan(
    plan: &ModuleLoadPlan,
) -> Result<Vec<VirtualMappingPlan>, ModuleLoadError> {
    let mut mappings = Vec::new();
    for segment in &plan.segments {
        let seg_end = segment
            .virtual_addr
            .checked_add(segment.mem_size)
            .ok_or(ModuleLoadError::SegmentAddressOverflow)?;

        let start = align_down(segment.virtual_addr, PAGE_SIZE);
        let end = align_up(seg_end, PAGE_SIZE).ok_or(ModuleLoadError::SegmentAddressOverflow)?;

        let zero_fill_bytes = segment.mem_size.saturating_sub(segment.file_size);
        mappings.push(VirtualMappingPlan {
            start,
            end,
            file_bytes: segment.file_size,
            zero_fill_bytes,
        });
    }

    mappings.sort_by_key(|m| m.start);
    for pair in mappings.windows(2) {
        let prev = pair[0];
        let next = pair[1];
        if prev.end > next.start {
            crate::klog_warn!(
                "[LOADER] mapping overlap prev=[{:#x},{:#x}) next=[{:#x},{:#x}) file_bytes=({}, {}) zero_fill=({}, {})",
                prev.start,
                prev.end,
                next.start,
                next.end,
                prev.file_bytes,
                next.file_bytes,
                prev.zero_fill_bytes,
                next.zero_fill_bytes,
            );
            return Err(ModuleLoadError::SegmentOverlap);
        }
    }

    Ok(mappings)
}

fn build_snapshot_from_parsed(
    elf: &ElfFile<'_>,
    image_len: usize,
) -> Result<ModuleImageSnapshot, ModuleLoadError> {
    let info = inspect_elf_from_parsed(elf);
    crate::kernel::debug_trace::record_optional(
        "loader.snapshot",
        "info_returned",
        Some(info.program_headers as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot info returned\n");
    let load_plan = build_load_plan_from_parsed(elf, image_len, &info)?;
    crate::kernel::debug_trace::record_optional(
        "loader.snapshot",
        "plan_returned",
        Some(load_plan.segments.len() as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot plan returned\n");
    let mappings = build_virtual_mapping_plan_from_load_plan(&load_plan)?;
    crate::kernel::debug_trace::record_optional(
        "loader.snapshot",
        "mappings_returned",
        Some(mappings.len() as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot mappings returned\n");

    Ok(ModuleImageSnapshot {
        info,
        load_plan,
        mappings,
    })
}

#[inline(always)]
fn prepare_snapshot_from_parsed(
    elf: &ElfFile<'_>,
    image_len: usize,
) -> Result<ModuleImageSnapshot, ModuleLoadError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snap build\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot build begin\n");
    build_snapshot_from_parsed(elf, image_len)
}

#[inline(always)]
fn prepare_snapshot_parse(image: &[u8]) -> Result<ElfFile<'_>, ModuleLoadError> {
    let elf = parse_elf(image)?;
    crate::kernel::debug_trace::record_optional(
        "loader.snapshot",
        "parsed_returned",
        Some(elf.header.pt2.ph_count() as u64),
        false,
    );
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snap parsed\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot parsed returned\n");
    Ok(elf)
}

#[inline(always)]
fn finish_snapshot_success(snapshot: ModuleImageSnapshot) -> ModuleImageSnapshot {
    PARSE_SUCCESS.fetch_add(1, Ordering::Relaxed);
    PLAN_SUCCESS.fetch_add(1, Ordering::Relaxed);
    MAP_PLAN_SUCCESS.fetch_add(1, Ordering::Relaxed);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snap ok\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot returned\n");
    snapshot
}

pub fn snapshot_module_image(image: &[u8]) -> Result<ModuleImageSnapshot, ModuleLoadError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snap begin\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot begin\n");
    PARSE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    PLAN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    MAP_PLAN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let elf = match prepare_snapshot_parse(image) {
        Ok(elf) => elf,
        Err(err) => {
            PARSE_FAILURES.fetch_add(1, Ordering::Relaxed);
            PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
            MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };
    let snapshot = match prepare_snapshot_from_parsed(&elf, image.len()) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            match err {
                ModuleLoadError::SegmentOverlap => {
                    PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
                    MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
                }
                ModuleLoadError::TooManyLoadSegments
                | ModuleLoadError::ImageTooLarge
                | ModuleLoadError::EntryOutsideLoadSegments
                | ModuleLoadError::NoLoadSegments
                | ModuleLoadError::SegmentOutOfBounds
                | ModuleLoadError::SegmentFileExceedsMem
                | ModuleLoadError::SegmentAlignmentMismatch
                | ModuleLoadError::SegmentAddressOverflow => {
                    PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
                    MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
                    MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
                }
            }
            return Err(err);
        }
    };

    Ok(finish_snapshot_success(snapshot))
}

pub fn preflight_module_image(image: &[u8]) -> Result<ModulePreflightReport, ModuleLoadError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader preflight begin\n");
    PREFLIGHT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let snapshot = match snapshot_module_image(image) {
        Ok(v) => v,
        Err(err) => {
            PREFLIGHT_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };

    let fingerprint = image_fingerprint(image);
    LAST_PREFLIGHT_FINGERPRINT.store(fingerprint, Ordering::Relaxed);
    PREFLIGHT_SUCCESS.fetch_add(1, Ordering::Relaxed);

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader preflight returned\n");
    Ok(ModulePreflightReport {
        entry: snapshot.info.entry,
        load_segments: snapshot.load_plan.segments.len(),
        total_file_bytes: snapshot.load_plan.total_file_bytes,
        total_mem_bytes: snapshot.load_plan.total_mem_bytes,
        fingerprint,
        machine: snapshot.info.machine,
    })
}

pub fn inspect_elf_image(image: &[u8]) -> Result<ModuleImageInfo, ModuleLoadError> {
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

pub fn materialize_load_segments(
    image: &[u8],
    load_plan: &ModuleLoadPlan,
) -> Result<u64, SegmentMaterializationError> {
    SEGMENT_MATERIALIZATION_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let mut total_bytes = 0u64;
    for segment in &load_plan.segments {
        if segment.file_size > segment.mem_size {
            SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(SegmentMaterializationError::InvalidSegmentRange);
        }

        let src_start = usize::try_from(segment.file_offset)
            .map_err(|_| SegmentMaterializationError::SegmentOutOfBounds)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let src_len = usize::try_from(segment.file_size)
            .map_err(|_| SegmentMaterializationError::SegmentOutOfBounds)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let src_end = src_start
            .checked_add(src_len)
            .ok_or(SegmentMaterializationError::SegmentOutOfBounds)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;

        if src_end > image.len() {
            SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(SegmentMaterializationError::SegmentOutOfBounds);
        }

        let dst = usize::try_from(segment.virtual_addr)
            .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let zero_fill = segment.mem_size - segment.file_size;
        let mem_size = usize::try_from(segment.mem_size)
            .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;
        let file_size = usize::try_from(segment.file_size)
            .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;

        let _ = dst
            .checked_add(mem_size)
            .ok_or(SegmentMaterializationError::SegmentAddressOverflow)
            .inspect_err(|_| {
                SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            })?;

        unsafe {
            core::ptr::copy_nonoverlapping(
                image.as_ptr().add(src_start),
                dst as *mut u8,
                file_size,
            );
            if zero_fill != 0 {
                let zero_len = usize::try_from(zero_fill)
                    .map_err(|_| SegmentMaterializationError::SegmentAddressOverflow)
                    .inspect_err(|_| {
                        SEGMENT_MATERIALIZATION_FAILURES.fetch_add(1, Ordering::Relaxed);
                    })?;
                core::ptr::write_bytes((dst as *mut u8).add(file_size), 0, zero_len);
            }
        }

        total_bytes = total_bytes
            .saturating_add(segment.file_size)
            .saturating_add(zero_fill);
    }

    SEGMENT_MATERIALIZED_BYTES.fetch_add(total_bytes, Ordering::Relaxed);
    SEGMENT_MATERIALIZATION_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(total_bytes)
}

pub fn build_virtual_mapping_plan(
    image: &[u8],
) -> Result<Vec<VirtualMappingPlan>, ModuleLoadError> {
    MAP_PLAN_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let plan = match build_load_plan(image) {
        Ok(p) => p,
        Err(err) => {
            MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(err);
        }
    };

    let mappings = build_virtual_mapping_plan_from_load_plan(&plan).inspect_err(|_| {
        MAP_PLAN_FAILURES.fetch_add(1, Ordering::Relaxed);
    })?;

    MAP_PLAN_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(mappings)
}
