use alloc::string::String;

#[path = "process_runtime/mappings.rs"]
mod mappings;
#[path = "process_runtime/tls.rs"]
mod tls;
#[path = "process_runtime/contract.rs"]
mod contract;
#[path = "process_runtime/trace.rs"]
mod trace;
#[path = "process_runtime/load_plan.rs"]
mod load_plan;

use super::{
    MappingRecord, Process, ProcessLifecycleState, ProcessRuntimeContractSnapshot,
};
use core::sync::atomic::Ordering;

#[inline(always)]
pub(super) fn bind_module_load_plan(
    process: &Process,
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<(), &'static str> {
    load_plan::bind_module_load_plan(process, plan)
}

#[inline(always)]
pub(super) fn bind_virtual_mappings(
    process: &Process,
    mappings: &[crate::kernel::module_loader::VirtualMappingPlan],
) -> Result<(), &'static str> {
    mappings::bind_virtual_mappings(process, mappings)
}

#[inline(always)]
pub fn bind_prepared_image_snapshot(
    process: &Process,
    image: &[u8],
    snapshot: &crate::kernel::module_loader::ModuleImageSnapshot,
) -> Result<(), &'static str> {
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "snapshot_begin",
        Some(snapshot.load_plan.entry),
        false,
    );
    trace::early_serial("[EARLY SERIAL] bind prepared snapshot begin\n");
    bind_module_load_plan(process, &snapshot.load_plan)?;
    mappings::bind_virtual_mappings(process, &snapshot.mappings)?;
    trace::early_serial("[EARLY SERIAL] bind prepared snapshot mappings returned\n");
    bind_tls_template(process, image, &snapshot.load_plan)?;
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "snapshot_returned",
        Some(snapshot.mappings.len() as u64),
        false,
    );
    trace::early_serial("[EARLY SERIAL] bind prepared snapshot returned\n");
    Ok(())
}

pub(super) fn allocate_user_vaddr(process: &Process, len: usize) -> Result<u64, &'static str> {
    if len == 0 {
        return Err("invalid length");
    }
    let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
    let pages = ((len as u64) + page_size - 1) / page_size;
    let bytes = pages.checked_mul(page_size).ok_or("overflow")?;
    let prev = process
        .next_mapping_hint
        .fetch_add(bytes, Ordering::Relaxed);
    let start = (prev + (page_size - 1)) & !(page_size - 1);
    Ok(start)
}

pub(super) fn register_mapping(
    process: &Process,
    map_id: u32,
    start: u64,
    end: u64,
    prot: u32,
    flags: u32,
) -> Result<(), &'static str> {
    if start >= end {
        return Err("invalid range");
    }
    let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
    if (start & (page_size - 1)) != 0 || (end & (page_size - 1)) != 0 {
        return Err("unaligned range");
    }
    let page_count = ((end - start) / page_size) as usize;

    let mut m = process.mappings.lock();
    m.push(MappingRecord {
        map_id,
        start,
        end,
        prot,
        flags,
    });
    process.mapped_regions.fetch_add(1, Ordering::Relaxed);
    process
        .mapped_pages
        .fetch_add(page_count, Ordering::Relaxed);
    Ok(())
}

pub(super) fn remove_mapping_record(process: &Process, map_id: u32) -> Option<MappingRecord> {
    let mut m = process.mappings.lock();
    if let Some(pos) = m.iter().position(|r| r.map_id == map_id) {
        let record = m.remove(pos);
        let page_size = crate::interfaces::memory::PAGE_SIZE_4K as u64;
        let page_count = ((record.end - record.start) / page_size) as usize;
        process.mapped_regions.fetch_sub(1, Ordering::Relaxed);
        process
            .mapped_pages
            .fetch_sub(page_count, Ordering::Relaxed);
        return Some(record);
    }
    None
}

pub(super) fn lifecycle_state(process: &Process) -> Option<ProcessLifecycleState> {
    ProcessLifecycleState::from_raw(process.lifecycle_state.load(Ordering::Relaxed))
}

pub(super) fn auxv_state(process: &Process) -> (usize, usize, usize, usize, usize, usize, usize) {
    (
        process.image_entry.load(Ordering::Relaxed),
        process.image_base.load(Ordering::Relaxed) as usize,
        process.image_phdr_addr.load(Ordering::Relaxed) as usize,
        process.image_phent_size.load(Ordering::Relaxed) as usize,
        process.image_phnum.load(Ordering::Relaxed) as usize,
        process.vdso_base.load(Ordering::Relaxed) as usize,
        process.vvar_base.load(Ordering::Relaxed) as usize,
    )
}

pub(super) fn set_exec_path(process: &Process, path: &str) {
    *process.exec_path.lock() = String::from(path);
}

pub(super) fn effective_entry(process: &Process) -> usize {
    let runtime_entry = process.runtime_entry.load(Ordering::Relaxed);
    if runtime_entry != 0 {
        runtime_entry as usize
    } else {
        process.image_entry.load(Ordering::Relaxed)
    }
}

#[inline(always)]
pub(super) fn bind_tls_template(
    process: &Process,
    image: &[u8],
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<(), &'static str> {
    tls::bind_tls_template(process, image, plan)
}

pub(super) fn runtime_contract_snapshot(process: &Process) -> ProcessRuntimeContractSnapshot {
    contract::runtime_contract_snapshot(process)
}

pub(super) fn append_deferred_fini_calls(process: &Process, fini_calls: &[u64]) {
    contract::append_deferred_fini_calls(process, fini_calls)
}

pub(super) fn clear_runtime_contract(process: &Process) {
    contract::clear_runtime_contract(process)
}
