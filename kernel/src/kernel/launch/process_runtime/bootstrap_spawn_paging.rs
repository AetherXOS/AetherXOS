//! # Safety
//!
//! All `unsafe` blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
#[cfg(feature = "paging_enable")]
use alloc::sync::Arc;
#[cfg(feature = "paging_enable")]
use crate::kernel::process::Process;

#[cfg(feature = "paging_enable")]
pub fn build_page_manager(
    process: &Arc<Process>,
) -> (x86_64::VirtAddr, crate::kernel::memory::paging::PageManager) {
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let offset = x86_64::VirtAddr::new(hhdm);
    let lvl4 = unsafe {
        &mut *(((process.cr3.as_u64() as u64) + hhdm) as *mut x86_64::structures::paging::PageTable)
    };
    let page_manager = crate::kernel::memory::paging::PageManager {
        mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
        physical_memory_offset: offset,
    };
    (offset, page_manager)
}

#[cfg(feature = "paging_enable")]
pub fn materialize_interpreter_image(
    process: &Arc<Process>,
    interpreter_image: Option<alloc::vec::Vec<u8>>,
    page_manager: &mut crate::kernel::memory::paging::PageManager,
    frame_allocator: &mut crate::hal::paging::PageAllocWrapper,
) -> Result<(), LaunchError> {
    if let Some(interp) = interpreter_image {
        let interp_prepared = crate::kernel::module_loader::materialize_process_image(
            process,
            &interp,
            page_manager,
            frame_allocator,
        )
        .map_err(|_| LaunchError::LoaderFailed)?;

        process.set_interpreter_base(interp_prepared.load_plan.aslr_base);
        process.set_runtime_entry(Some(
            interp_prepared.load_plan.entry + interp_prepared.load_plan.aslr_base,
        ));
    }

    Ok(())
}

