use super::{effective_entry, Process, ProcessRuntimeContractSnapshot};
use crate::kernel::process::RuntimeLifecycleHooks;

pub(super) fn runtime_contract_snapshot(
    process: &Process,
) -> ProcessRuntimeContractSnapshot {
    let hooks = process.runtime_hooks_snapshot();
    ProcessRuntimeContractSnapshot {
        image_entry: process
            .image_entry
            .load(core::sync::atomic::Ordering::Relaxed),
        runtime_entry: effective_entry(process),
        runtime_fini_entry: process
            .runtime_fini_entry
            .load(core::sync::atomic::Ordering::Relaxed) as usize,
        image_base: process.image_base.load(core::sync::atomic::Ordering::Relaxed) as usize,
        phdr_addr: process
            .image_phdr_addr
            .load(core::sync::atomic::Ordering::Relaxed) as usize,
        vdso_base: process.vdso_base.load(core::sync::atomic::Ordering::Relaxed) as usize,
        vvar_base: process.vvar_base.load(core::sync::atomic::Ordering::Relaxed) as usize,
        exec_path: process.exec_path_snapshot(),
        init_calls: hooks.ordered_init_calls(),
        fini_calls: hooks.ordered_fini_calls(),
    }
}

pub(super) fn append_deferred_fini_calls(process: &Process, fini_calls: &[u64]) {
    if fini_calls.is_empty() {
        return;
    }
    let mut hooks = process.runtime_hooks.lock();
    for addr in fini_calls.iter().copied().filter(|addr| *addr != 0) {
        if !hooks.deferred_fini.iter().any(|existing| *existing == addr) {
            hooks.deferred_fini.push(addr);
        }
    }
}

pub(super) fn clear_runtime_contract(process: &Process) {
    process
        .runtime_entry
        .store(0, core::sync::atomic::Ordering::Relaxed);
    process
        .runtime_fini_entry
        .store(0, core::sync::atomic::Ordering::Relaxed);
    process.set_runtime_hooks(RuntimeLifecycleHooks::default());
}
