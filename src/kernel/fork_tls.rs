use super::*;

#[cfg(all(feature = "ring_protection", feature = "posix_mman"))]
pub(super) fn initialize_task_tls(
    process: &crate::kernel::process::Process,
    task: &mut KernelTask,
) -> Result<(), ForkError> {
    const TCB_BYTES: usize = 16;
    const DTV_BYTES: usize = 16;

    let (template, mem_size, align) = process.tls_state_snapshot();
    if mem_size == 0 {
        task.user_tls_base = 0;
        return Ok(());
    }

    let tls_data_len = usize::try_from(mem_size.max(template.len() as u64))
        .map_err(|_| ForkError::LimitExceeded)?;
    let tls_align = usize::try_from(align.max(1)).map_err(|_| ForkError::LimitExceeded)?;
    let tls_span = tls_data_len
        .checked_add(tls_align - 1)
        .ok_or(ForkError::LimitExceeded)?
        / tls_align
        * tls_align;
    let total_len = tls_span
        .checked_add(TCB_BYTES)
        .and_then(|v| v.checked_add(DTV_BYTES))
        .ok_or(ForkError::LimitExceeded)?;
    let map_id = crate::modules::posix::mman::mmap_anonymous(
        total_len,
        crate::modules::posix_consts::mman::PROT_READ
            | crate::modules::posix_consts::mman::PROT_WRITE,
        crate::modules::posix_consts::mman::MAP_PRIVATE,
    )
    .map_err(|_| ForkError::LimitExceeded)?;
    let tls_start = process
        .allocate_user_vaddr(total_len)
        .map_err(|_| ForkError::LimitExceeded)?;
    let mapping_end = tls_start
        .checked_add(total_len as u64)
        .ok_or(ForkError::LimitExceeded)?;
    let tcb_start = tls_start
        .checked_add(tls_span as u64)
        .ok_or(ForkError::LimitExceeded)?;
    let dtv_start = tcb_start
        .checked_add(TCB_BYTES as u64)
        .ok_or(ForkError::LimitExceeded)?;
    process
        .register_mapping(
            map_id,
            tls_start,
            mapping_end,
            (crate::modules::posix_consts::mman::PROT_READ
                | crate::modules::posix_consts::mman::PROT_WRITE) as u32,
            crate::modules::posix_consts::mman::MAP_PRIVATE as u32,
        )
        .map_err(|_| ForkError::LimitExceeded)?;

    crate::kernel::syscalls::with_user_write_bytes(tls_start as usize, total_len, |dst| {
        dst.fill(0);
        dst[..template.len()].copy_from_slice(&template);
        let tcb_off = tls_span;
        dst[tcb_off..tcb_off + 8].copy_from_slice(&tcb_start.to_le_bytes());
        dst[tcb_off + 8..tcb_off + 16].copy_from_slice(&dtv_start.to_le_bytes());
        let dtv_off = tcb_off + TCB_BYTES;
        dst[dtv_off..dtv_off + 8].copy_from_slice(&1u64.to_le_bytes());
        dst[dtv_off + 8..dtv_off + 16].copy_from_slice(&tls_start.to_le_bytes());
    })
    .map_err(|_| ForkError::LimitExceeded)?;

    task.user_tls_base = tcb_start;
    Ok(())
}

#[cfg(not(all(feature = "ring_protection", feature = "posix_mman")))]
#[allow(dead_code)]
pub(super) fn initialize_task_tls(
    _process: &crate::kernel::process::Process,
    _task: &mut KernelTask,
) -> Result<(), ForkError> {
    Ok(())
}
