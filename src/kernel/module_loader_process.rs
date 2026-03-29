use super::*;

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
mod dynamic_link;

#[inline(always)]
fn bind_snapshot_to_process(
    process: &crate::kernel::process::Process,
    image: &[u8],
    snapshot: ModuleImageSnapshot,
) -> Result<ModuleImageSnapshot, ProcessPrepareError> {
    crate::kernel::debug_trace::record_optional(
        "loader.prepare",
        "bind_begin",
        Some(snapshot.mappings.len() as u64),
        false,
    );
    crate::kernel::process::bind_prepared_image_snapshot(process, image, &snapshot)
        .map_err(|err| {
            crate::kernel::debug_trace::record_fault("loader.prepare", "bind_failed", None);
            crate::klog_warn!("[LOADER] bind_prepared_image_snapshot failed: {}", err);
            ProcessPrepareError::ProcessBindFailed
        })?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader bind prepared snapshot returned\n",
    );
    crate::kernel::debug_trace::record_optional(
        "loader.prepare",
        "bind_returned",
        Some(snapshot.load_plan.segments.len() as u64),
        false,
    );

    Ok(snapshot)
}

#[inline(always)]
pub fn prepare_process_image_entry_from_snapshot(
    process: &crate::kernel::process::Process,
    image: &[u8],
    snapshot: ModuleImageSnapshot,
) -> Result<u64, ProcessPrepareError> {
    let snapshot = bind_snapshot_to_process(process, image, snapshot)?;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::x86_64::serial::write_raw(
        "[EARLY SERIAL] loader prepare process image entry returned\n",
    );
    crate::kernel::debug_trace::record_with_metadata(
        "loader.prepare",
        "entry_returned",
        Some(snapshot.load_plan.entry),
        false,
        crate::kernel::debug_trace::TraceSeverity::Trace,
        crate::kernel::debug_trace::TraceCategory::Loader,
    );
    Ok(snapshot.load_plan.entry)
}

#[inline(always)]
fn snapshot_and_bind_process_image(
    process: &crate::kernel::process::Process,
    image: &[u8],
) -> Result<ModuleImageSnapshot, ProcessPrepareError> {
    crate::kernel::debug_trace::record_optional(
        "loader.prepare",
        "snapshot_begin",
        Some(image.len() as u64),
        false,
    );
    let snapshot = snapshot_module_image(image).map_err(|err| {
        crate::kernel::debug_trace::record_fault("loader.prepare", "snapshot_failed", Some(err as u64));
        crate::klog_warn!("[LOADER] snapshot_module_image failed: {:?}", err);
        ProcessPrepareError::Loader(err)
    })?;
    crate::kernel::debug_trace::record_optional(
        "loader.prepare",
        "snapshot_returned",
        Some(snapshot.load_plan.entry),
        false,
    );
    bind_snapshot_to_process(process, image, snapshot)
}

#[inline(always)]
pub fn prepare_process_image_entry(
    process: &crate::kernel::process::Process,
    image: &[u8],
) -> Result<u64, ProcessPrepareError> {
    crate::kernel::debug_trace::record_optional(
        "loader.prepare",
        "snapshot_begin",
        Some(image.len() as u64),
        false,
    );
    let snapshot = snapshot_module_image(image).map_err(|err| {
        crate::kernel::debug_trace::record_fault("loader.prepare", "snapshot_failed", Some(err as u64));
        crate::klog_warn!("[LOADER] snapshot_module_image failed: {:?}", err);
        ProcessPrepareError::Loader(err)
    })?;
    crate::kernel::debug_trace::record_optional(
        "loader.prepare",
        "snapshot_returned",
        Some(snapshot.load_plan.entry),
        false,
    );
    prepare_process_image_entry_from_snapshot(process, image, snapshot)
}

#[inline(always)]
pub fn prepare_process_image(
    process: &crate::kernel::process::Process,
    image: &[u8],
) -> Result<alloc::boxed::Box<PreparedProcessImage>, ProcessPrepareError> {
    let snapshot = snapshot_and_bind_process_image(process, image)?;

    Ok(alloc::boxed::Box::new(PreparedProcessImage {
        info: snapshot.info,
        load_plan: snapshot.load_plan,
        mappings: snapshot.mappings,
    }))
}

#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub fn materialize_process_image(
    process: &crate::kernel::process::Process,
    image: &[u8],
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
        x86_64::structures::paging::Size4KiB,
    >,
) -> Result<alloc::boxed::Box<PreparedProcessImage>, ProcessPrepareError> {
    use x86_64::structures::paging::PageTableFlags;

    let mut prepared = prepare_process_image(process, image)?;

    let flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

    page_manager
        .apply_virtual_mapping_plan(&prepared.mappings, flags, frame_allocator)
        .map_err(|_| ProcessPrepareError::PagingApplyFailed)?;

    materialize_load_segments(image, &prepared.load_plan)
        .map_err(|_| ProcessPrepareError::SegmentMaterializationFailed)?;

    dynamic_link::apply_dynamic_linking_and_runtime_trampolines(
        process,
        image,
        &mut prepared,
        page_manager,
        frame_allocator,
    );

    Ok(prepared)
}
