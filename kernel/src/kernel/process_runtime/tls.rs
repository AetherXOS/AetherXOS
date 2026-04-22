use super::{trace, Process};

#[inline(always)]
fn tls_template_mut(process: &Process) -> &mut alloc::vec::Vec<u8> {
    // Safety: bootstrap_borrow_mut is only used here on the process-owned TLS cell
    // during runtime contract binding paths where exclusive mutation is intended.
    unsafe { process.tls_template.bootstrap_borrow_mut() }
}

fn publish_tls_header(process: &Process, plan: &crate::kernel::module_loader::ModuleLoadPlan) {
    trace::early_serial("[EARLY SERIAL] tls header publish begin\n");
    process
        .tls_mem_size
        .store(plan.tls_mem_size, core::sync::atomic::Ordering::Relaxed);
    process
        .tls_align
        .store(plan.tls_align.max(1), core::sync::atomic::Ordering::Relaxed);
    trace::early_serial("[EARLY SERIAL] tls header publish returned\n");
}

fn validate_tls_contract(
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
    template: &[u8],
) -> Result<(), &'static str> {
    if plan.tls_mem_size == 0 {
        return if plan.tls_file_size == 0 && template.is_empty() {
            Ok(())
        } else {
            Err("tls empty plan mismatch")
        };
    }

    if plan.tls_file_size > plan.tls_mem_size {
        return Err("tls file size exceeds memory size");
    }
    if plan.tls_align > 1 && !plan.tls_align.is_power_of_two() {
        return Err("tls alignment must be power of two");
    }
    if usize::try_from(plan.tls_file_size).ok() != Some(template.len()) {
        return Err("tls template length mismatch");
    }
    Ok(())
}

fn clear_tls_template(process: &Process) {
    trace::early_serial("[EARLY SERIAL] tls bootstrap borrow begin\n");
    let tls = tls_template_mut(process);
    tls.clear();
    trace::early_serial("[EARLY SERIAL] tls bootstrap borrow returned\n");
}

fn compute_tls_file_range(
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<(usize, usize), &'static str> {
    trace::early_serial("[EARLY SERIAL] tls vaddr calc begin\n");
    let tls_vaddr = plan
        .tls_virtual_addr
        .checked_sub(plan.aslr_base)
        .ok_or("tls virtual address underflow")?;
    trace::early_serial("[EARLY SERIAL] tls vaddr calc returned\n");
    trace::early_serial("[EARLY SERIAL] tls segment lookup begin\n");
    let segment = plan
        .segments
        .iter()
        .find(|segment| {
            let seg_end = segment
                .virtual_addr
                .checked_add(segment.mem_size)
                .unwrap_or(0);
            plan.tls_virtual_addr >= segment.virtual_addr && plan.tls_virtual_addr < seg_end
        })
        .ok_or("tls segment not covered by load segment")?;
    trace::early_serial("[EARLY SERIAL] tls segment lookup returned\n");
    trace::early_serial("[EARLY SERIAL] tls delta calc begin\n");
    let delta = tls_vaddr
        .checked_sub(
            segment
                .virtual_addr
                .checked_sub(plan.aslr_base)
                .ok_or("segment base underflow")?,
        )
        .ok_or("tls segment delta underflow")?;
    trace::early_serial("[EARLY SERIAL] tls delta calc returned\n");
    trace::early_serial("[EARLY SERIAL] tls file range begin\n");
    let file_offset = segment
        .file_offset
        .checked_add(delta)
        .ok_or("tls file offset overflow")? as usize;
    let file_size = usize::try_from(plan.tls_file_size).map_err(|_| "tls file size overflow")?;
    let file_end = file_offset
        .checked_add(file_size)
        .ok_or("tls file range overflow")?;
    trace::early_serial("[EARLY SERIAL] tls file range returned\n");
    Ok((file_offset, file_end))
}

fn build_tls_template_bytes(
    image: &[u8],
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<alloc::vec::Vec<u8>, &'static str> {
    let (file_offset, file_end) = compute_tls_file_range(plan)?;
    trace::early_serial("[EARLY SERIAL] tls image slice begin\n");
    let bytes = image
        .get(file_offset..file_end)
        .ok_or("tls image bytes out of bounds")?;
    crate::kernel::debug_trace::record_bytes_preview("process.tls", "image_preview", bytes);
    trace::early_serial("[EARLY SERIAL] tls image slice returned\n");

    trace::early_serial("[EARLY SERIAL] tls local vec begin\n");
    let reserve_len = usize::try_from(plan.tls_mem_size).map_err(|_| "tls memory size overflow")?;
    let mut next_tls = alloc::vec::Vec::with_capacity(reserve_len);
    next_tls.extend_from_slice(bytes);
    trace::early_serial("[EARLY SERIAL] tls local vec returned\n");
    Ok(next_tls)
}

fn publish_tls_template_bytes(
    process: &Process,
    next_tls: alloc::vec::Vec<u8>,
    tls_mem_size: u64,
) {
    trace::early_serial("[EARLY SERIAL] tls bootstrap borrow begin\n");
    let tls = tls_template_mut(process);
    *tls = next_tls;
    crate::kernel::debug_trace::record_kernel_context(
        "process.bind",
        "tls_returned",
        Some(tls_mem_size),
    );
    trace::early_serial("[EARLY SERIAL] tls bootstrap borrow returned\n");
}

#[inline(always)]
pub(super) fn bind_tls_template(
    process: &Process,
    image: &[u8],
    plan: &crate::kernel::module_loader::ModuleLoadPlan,
) -> Result<(), &'static str> {
    crate::kernel::debug_trace::record_optional(
        "process.bind",
        "tls_begin",
        Some(plan.tls_mem_size),
        false,
    );
    trace::early_serial("[EARLY SERIAL] bind tls template begin\n");

    if plan.tls_mem_size == 0 {
        clear_tls_template(process);
        crate::kernel::debug_trace::record_with_metadata(
            "process.bind",
            "tls_empty",
            Some(0),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Memory,
        );
        trace::early_serial("[EARLY SERIAL] tls empty shortcut returned\n");
        return Ok(());
    }

    let next_tls = build_tls_template_bytes(image, plan)?;

    validate_tls_contract(plan, &next_tls)?;
    publish_tls_header(process, plan);

    publish_tls_template_bytes(process, next_tls, plan.tls_mem_size);
    trace::early_serial("[EARLY SERIAL] bind tls template returned\n");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_process() -> Process {
        Process::new(
            b"tls-test",
            #[cfg(feature = "paging_enable")]
            x86_64::PhysAddr::new(0),
        )
    }

    fn test_plan(
        tls_file_size: u64,
        tls_mem_size: u64,
        tls_align: u64,
    ) -> crate::kernel::module_loader::ModuleLoadPlan {
        crate::kernel::module_loader::ModuleLoadPlan {
            entry: 0x401000,
            segments: alloc::vec![crate::kernel::module_loader::LoadSegmentPlan {
                virtual_addr: 0x1000,
                file_offset: 0,
                file_size: tls_file_size,
                mem_size: tls_mem_size,
                align: tls_align,
            }],
            total_file_bytes: tls_file_size,
            total_mem_bytes: tls_mem_size,
            aslr_base: 0,
            tls_virtual_addr: 0x1000,
            tls_file_size,
            tls_mem_size,
            tls_align,
            program_header_addr: 0,
            program_header_entry_size: 0,
            program_headers: 0,
        }
    }

    #[test_case]
    fn validate_tls_contract_accepts_power_of_two_alignment() {
        let plan = test_plan(4, 8, 8);
        let template = alloc::vec![1, 2, 3, 4];

        assert!(validate_tls_contract(&plan, &template).is_ok());
    }

    #[test_case]
    fn validate_tls_contract_rejects_file_size_overflow() {
        let plan = test_plan(8, 4, 8);
        let template = alloc::vec![1, 2, 3, 4, 5, 6, 7, 8];

        assert_eq!(validate_tls_contract(&plan, &template), Err("tls file size exceeds memory size"));
    }

    #[test_case]
    fn bind_tls_template_rejects_invalid_alignment() {
        let process = test_process();
        let plan = test_plan(4, 8, 3);
        let image = alloc::vec![0u8; 4];

        assert_eq!(bind_tls_template(&process, &image, &plan), Err("tls alignment must be power of two"));
    }
}
