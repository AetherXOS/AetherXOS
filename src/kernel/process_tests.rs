use super::{Process, RuntimeLifecycleHooks};

#[test_case]
fn runtime_hooks_order_init_and_fini_calls_as_expected() {
    let hooks = RuntimeLifecycleHooks {
        preinit_array: alloc::vec![0x10, 0, 0x20],
        init: Some(0x30),
        init_array: alloc::vec![0x40, 0x50],
        fini_array: alloc::vec![0x60, 0, 0x70],
        fini: Some(0x80),
    };

    assert_eq!(
        hooks.ordered_init_calls(),
        alloc::vec![0x10, 0x20, 0x30, 0x40, 0x50]
    );
    assert_eq!(hooks.ordered_fini_calls(), alloc::vec![0x70, 0x60, 0x80]);
}

#[test_case]
fn runtime_contract_snapshot_tracks_exec_path_and_hook_counts() {
    let process = Process::new(
        b"test",
        #[cfg(feature = "paging_enable")]
        x86_64::PhysAddr::new(0),
    );
    process.set_exec_path("/usr/lib/hypercore/init");
    process.set_runtime_hooks(RuntimeLifecycleHooks {
        preinit_array: alloc::vec![0x1000],
        init: Some(0x2000),
        init_array: alloc::vec![0x3000],
        fini_array: alloc::vec![0x4000, 0x5000],
        fini: Some(0x6000),
    });
    process.set_runtime_entry(Some(0x7777));
    process.set_runtime_fini_entry(Some(0x8888));
    let snapshot = process.runtime_contract_snapshot();
    assert_eq!(snapshot.exec_path, "/usr/lib/hypercore/init");
    assert_eq!(snapshot.runtime_entry, 0x7777);
    assert_eq!(snapshot.runtime_fini_entry, 0x8888);
    assert_eq!(snapshot.init_calls, alloc::vec![0x1000, 0x2000, 0x3000]);
    assert_eq!(snapshot.fini_calls, alloc::vec![0x5000, 0x4000, 0x6000]);
}

#[test_case]
fn runtime_hooks_deduplicate_fini_calls_across_deferred_and_static_hooks() {
    let hooks = RuntimeLifecycleHooks {
        deferred_fini: alloc::vec![0x9000, 0x9000, 0x7000],
        fini_array: alloc::vec![0x7000, 0x8000, 0x9000],
        fini: Some(0x8000),
        ..RuntimeLifecycleHooks::default()
    };

    assert_eq!(hooks.ordered_fini_calls(), alloc::vec![0x9000, 0x7000, 0x8000]);
}

#[test_case]
fn effective_entry_falls_back_to_image_entry_when_runtime_entry_is_cleared() {
    let process = Process::new(
        b"test",
        #[cfg(feature = "paging_enable")]
        x86_64::PhysAddr::new(0),
    );

    let load_plan = crate::kernel::module_loader::ModuleLoadPlan {
        entry: 0x401000,
        segments: alloc::vec![],
        total_file_bytes: 0,
        total_mem_bytes: 0,
        aslr_base: 0,
        tls_virtual_addr: 0,
        tls_file_size: 0,
        tls_mem_size: 0,
        tls_align: 0,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };

    process
        .bind_module_load_plan(&load_plan)
        .expect("bind module load plan");
    process.set_runtime_entry(Some(0x7777));
    assert_eq!(process.effective_entry(), 0x7777);

    process.set_runtime_entry(None);
    assert_eq!(process.effective_entry(), 0x401000);
}

#[test_case]
fn auxv_state_reflects_bound_module_plan_fields() {
    let process = Process::new(
        b"auxv",
        #[cfg(feature = "paging_enable")]
        x86_64::PhysAddr::new(0),
    );

    let load_plan = crate::kernel::module_loader::ModuleLoadPlan {
        entry: 0x500000,
        segments: alloc::vec![],
        total_file_bytes: 0,
        total_mem_bytes: 0,
        aslr_base: 0x200000,
        tls_virtual_addr: 0,
        tls_file_size: 0,
        tls_mem_size: 0,
        tls_align: 0,
        program_header_addr: 0x501000,
        program_header_entry_size: 56,
        program_headers: 9,
    };

    process
        .bind_module_load_plan(&load_plan)
        .expect("bind module load plan");

    let (entry, base, phdr, phent, phnum, _vdso, _vvar) = process.auxv_state();
    assert_eq!(entry, 0x500000);
    assert_eq!(base, 0x200000);
    assert_eq!(phdr, 0x501000);
    assert_eq!(phent, 56);
    assert_eq!(phnum, 9);
}
