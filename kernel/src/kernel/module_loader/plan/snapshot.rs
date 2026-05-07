use super::super::*;
use super::elf::{inspect_elf_from_parsed, parse_elf};
use super::load::build_load_plan_from_parsed;
use super::mapping::build_virtual_mapping_plan_from_load_plan;
use xmas_elf::ElfFile;
use super::super::{ModuleImageSnapshot, ModuleLoadError, PARSE_SUCCESS, PLAN_SUCCESS, MAP_PLAN_SUCCESS, PARSE_ATTEMPTS, PLAN_ATTEMPTS, MAP_PLAN_ATTEMPTS, PARSE_FAILURES, PLAN_FAILURES, MAP_PLAN_FAILURES};
use core::sync::atomic::Ordering;
use super::super::support::image_fingerprint;

fn build_snapshot_from_parsed(
    elf: &ElfFile<'_>,
    image_len: usize,
    fingerprint: u64,
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
        fingerprint,
        entry: load_plan.entry,
        info,
        load_plan: alloc::sync::Arc::new(load_plan),
        mappings,
    })
}

#[inline(always)]
fn prepare_snapshot_from_parsed(
    elf: &ElfFile<'_>,
    image_len: usize,
    fingerprint: u64,
) -> Result<ModuleImageSnapshot, ModuleLoadError> {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snap build\n");
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader snapshot build begin\n");
    build_snapshot_from_parsed(elf, image_len, fingerprint)
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
    let fingerprint = image_fingerprint(image);
    let snapshot = match prepare_snapshot_from_parsed(&elf, image.len(), fingerprint) {
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

    LAST_PREFLIGHT_FINGERPRINT.store(snapshot.fingerprint, Ordering::Relaxed);
    PREFLIGHT_SUCCESS.fetch_add(1, Ordering::Relaxed);

    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] loader preflight returned\n");
    Ok(ModulePreflightReport {
        entry: snapshot.info.entry,
        load_segments: snapshot.load_plan.segments.len(),
        total_file_bytes: snapshot.load_plan.total_file_bytes,
        total_mem_bytes: snapshot.load_plan.total_mem_bytes,
        fingerprint: snapshot.fingerprint,
        machine: snapshot.info.machine,
    })
}
