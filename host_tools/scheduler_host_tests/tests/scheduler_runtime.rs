use hypercore::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use hypercore::kernel::launch::{
    clone_process_from_registered_image, launch_registry_snapshot, process_boot_image,
    process_count, spawn_bootstrap_from_aligned_static_image, spawn_bootstrap_from_image,
    spawn_bootstrap_from_static_image,
    LaunchError, LaunchRegistrySnapshotEntry,
};
use hypercore::kernel::module_loader::{
    build_load_plan, build_process_bootstrap_task, build_process_bootstrap_task_from_snapshot,
    build_virtual_mapping_plan, inspect_elf_image, preflight_module_image,
    prepare_process_image_entry_from_snapshot,
    prepare_process_image, prepare_process_image_entry, snapshot_module_image,
};
use hypercore::kernel::module_loader::ModuleLoadPlan;
use hypercore::kernel::module_loader::LoadSegmentPlan;
use hypercore::kernel::module_loader::ModuleLoadError;
use hypercore::kernel::process::{Process, ProcessLifecycleState};
use hypercore::kernel::sync::IrqSafeMutex;
use hypercore::kernel::task::{
    get_task, register_task_arc, task_context_snapshot, task_ids_snapshot, task_registry_snapshot,
    unregister_task,
};
use hypercore::kernel::debug_trace::{
    latest_record, recent_records_for_category, recent_records_vec_copy,
    record_with_metadata, TraceCategory, TraceSeverity, TraceRecord,
};
use hypercore::modules::schedulers::{
    cfs::CFS, cooperative::Cooperative, edf::EDF, fifo::FIFO, lifo::LIFO, lottery::Lottery,
    mlfq::MLFQ, round_robin::RoundRobin, weighted_round_robin::WeightedRoundRobin,
};
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicUsize, Ordering as StdOrdering};

extern "C" fn host_test_entry() -> ! {
    loop {}
}

static PROBE_LINKED_ELF: &[u8] =
    include_bytes!("../../../boot/initramfs/usr/lib/hypercore/probe-linked.elf");
static TRACE_TEST_GUARD: Mutex<()> = Mutex::new(());
static TASK_REGISTRY_TEST_GUARD: Mutex<()> = Mutex::new(());
static TRACE_SCOPE_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn trace_test_guard() -> MutexGuard<'static, ()> {
    TRACE_TEST_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn task_registry_test_guard() -> MutexGuard<'static, ()> {
    TASK_REGISTRY_TEST_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn aligned_probe_linked_elf() -> Box<[u64]> {
    let words = PROBE_LINKED_ELF.len().div_ceil(core::mem::size_of::<u64>());
    let mut storage = vec![0u64; words].into_boxed_slice();
    let byte_len = storage.len() * core::mem::size_of::<u64>();
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(storage.as_mut_ptr() as *mut u8, byte_len)
    };
    bytes[..PROBE_LINKED_ELF.len()].copy_from_slice(PROBE_LINKED_ELF);
    storage
}

fn unique_trace_scope(prefix: &str) -> String {
    let id = TRACE_SCOPE_COUNTER.fetch_add(1, StdOrdering::Relaxed);
    format!("{prefix}_{id}")
}

fn aligned_probe_linked_elf_bytes() -> Vec<u8> {
    let storage = aligned_probe_linked_elf();
    let byte_len = storage.len() * core::mem::size_of::<u64>();
    let bytes = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, byte_len)
    };
    bytes[..PROBE_LINKED_ELF.len()].to_vec()
}

fn unsupported_class_probe_linked_elf_bytes() -> Vec<u8> {
    let mut image = aligned_probe_linked_elf_bytes();
    image[4] = 1;
    image
}

fn make_task(id: usize, priority: u8) -> KernelTask {
    let mut stack = vec![0u64; 32].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let task = KernelTask::new(
        TaskId(id),
        priority,
        0,
        0,
        top as u64,
        0,
        host_test_entry as *const () as usize as u64,
    );
    Box::leak(stack);
    task
}

fn make_arc_task(id: usize, priority: u8) -> Arc<IrqSafeMutex<KernelTask>> {
    Arc::new(IrqSafeMutex::new(make_task(id, priority)))
}

fn make_process(name: &[u8]) -> Process {
    Process::new(name)
}

fn make_bootstrap_process(name: &[u8]) -> Process {
    Process::new_bootstrap(name)
}

fn assert_bootstrap_task_contract(
    process: &Process,
    task: &Arc<IrqSafeMutex<KernelTask>>,
    expected_tid: TaskId,
) {
    let runtime = process.runtime_contract_snapshot();
    let threads = process.threads.lock();
    let task = task.lock();
    assert_eq!(threads.as_slice().last().copied(), Some(expected_tid));
    assert_eq!(task.id, expected_tid);
    assert_eq!(task.context.rsp, task.kernel_stack_pointer);
    assert_eq!(task.context.rip, runtime.runtime_entry as u64);
    assert_eq!(task.page_table_root, 0);
    assert_ne!(runtime.runtime_entry, 0);
    assert_ne!(task.kernel_stack_pointer, 0);
}

#[test]
fn x86_64_initial_stack_frame_matches_context_switch_contract() {
    let mut stack = vec![0u64; 16].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let entry = host_test_entry as *const () as usize as u64;
    let task = KernelTask::new(TaskId(41), 0, 0, 0, top as u64, 0, entry);

    let frame_words = &stack[stack.len() - 8..];
    assert_eq!(task.kernel_stack_pointer as usize, top - 8 * core::mem::size_of::<u64>());
    assert_eq!(frame_words[0], 0);
    assert_eq!(frame_words[1], 0);
    assert_eq!(frame_words[2], 0);
    assert_eq!(frame_words[3], 0);
    assert_eq!(frame_words[4], 0);
    assert_eq!(frame_words[5], 0);
    assert_eq!(frame_words[6], entry);
    assert_ne!(frame_words[7], 0);
}

#[test]
fn x86_64_initial_stack_frame_repeated_layout_stays_exact() {
    let entry = host_test_entry as *const () as usize as u64;

    for words in [16usize, 24, 32, 48, 64] {
        let mut stack = vec![0u64; words].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let task = KernelTask::new(TaskId(3000 + words), 1, 2, 3, top as u64, 0, entry);

        let frame_words = &stack[stack.len() - 8..];
        assert_eq!(task.kernel_stack_pointer as usize, top - 8 * core::mem::size_of::<u64>());
        assert_eq!(frame_words[0..6], [0, 0, 0, 0, 0, 0]);
        assert_eq!(frame_words[6], entry);
        assert_ne!(frame_words[7], 0);
    }
}

#[test]
fn x86_64_initial_stack_frame_image_matches_expected_contract() {
    let entry = host_test_entry as *const () as usize as u64;
    let mut stack = vec![0u64; 16].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let task = KernelTask::new(TaskId(3333), 1, 2, 3, top as u64, 0, entry);
    let frame_words = &stack[stack.len() - 8..];

    assert_eq!(task.context.rip, entry);
    assert_eq!(frame_words[0..6], [0, 0, 0, 0, 0, 0]);
    assert_eq!(frame_words[6], entry);
    assert_ne!(frame_words[7], 0);
}

#[test]
fn x86_64_initial_stack_frame_handles_small_stack_without_underflowing() {
    let entry = host_test_entry as *const () as usize as u64;
    let task = KernelTask::new(TaskId(42), 0, 0, 0, 32, 0, entry);
    assert_eq!(task.kernel_stack_pointer, 0);
    assert_eq!(task.context.rsp, 0);
}

#[test]
fn x86_64_new_shared_preserves_prepared_stack_contract() {
    let mut stack = vec![0u64; 16].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let entry = host_test_entry as *const () as usize as u64;
    let task = KernelTask::new_shared(TaskId(43), 0, 0, 0, top as u64, 0, entry);
    let task = task.lock();

    assert_eq!(task.kernel_stack_pointer as usize, top - 8 * core::mem::size_of::<u64>());
    assert_eq!(task.context.rsp, task.kernel_stack_pointer);
    assert_eq!(task.context.rip, entry);
}

#[test]
fn x86_64_new_shared_repeated_bootstrap_shapes_stay_consistent() {
    let entry = host_test_entry as *const () as usize as u64;

    for words in [16usize, 24, 32, 64, 128] {
        let mut stack = vec![0u64; words].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let task = KernelTask::new_shared(TaskId(1000 + words), 0, 0, 0, top as u64, 0, entry);
        let task = task.lock();

        assert_ne!(task.kernel_stack_pointer, 0);
        assert_eq!(task.context.rsp, task.kernel_stack_pointer);
        assert_eq!(task.context.rip, entry);
        assert_eq!(task.kernel_stack_pointer & 0xF, 0);
    }
}

#[test]
fn x86_64_new_from_spec_matches_direct_constructor_contract() {
    let mut stack = vec![0u64; 24].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let entry = host_test_entry as *const () as usize as u64;
    let spec = KernelTask::bootstrap_spec(TaskId(44), 3, 7, 11, top as u64, 0x1234, entry);

    let direct = KernelTask::new(TaskId(44), 3, 7, 11, top as u64, 0x1234, entry);
    let from_spec = KernelTask::new_from_spec(spec);

    assert_eq!(from_spec.id, direct.id);
    assert_eq!(from_spec.priority, direct.priority);
    assert_eq!(from_spec.deadline, direct.deadline);
    assert_eq!(from_spec.burst_time, direct.burst_time);
    assert_eq!(from_spec.page_table_root, direct.page_table_root);
    assert_eq!(from_spec.context.rip, direct.context.rip);
    assert_eq!(from_spec.context.rsp, direct.context.rsp);
    assert_eq!(from_spec.kernel_stack_pointer, direct.kernel_stack_pointer);
}

#[test]
fn x86_64_new_shared_from_spec_preserves_shared_bootstrap_contract() {
    let mut stack = vec![0u64; 32].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let entry = host_test_entry as *const () as usize as u64;
    let spec = KernelTask::bootstrap_spec(TaskId(45), 2, 5, 9, top as u64, 0x5678, entry);

    let task = KernelTask::new_shared_from_spec(spec);
    let task = task.lock();

    assert_eq!(task.id, TaskId(45));
    assert_eq!(task.priority, 2);
    assert_eq!(task.deadline, 5);
    assert_eq!(task.burst_time, 9);
    assert_eq!(task.page_table_root, 0x5678);
    assert_eq!(task.context.rip, entry);
    assert_eq!(task.context.rsp, task.kernel_stack_pointer);
    assert_ne!(task.kernel_stack_pointer, 0);
}

#[test]
fn x86_64_new_shared_from_spec_repeated_bootstrap_publish_is_stable() {
    let entry = host_test_entry as *const () as usize as u64;

    for i in 0..32usize {
        let mut stack = vec![0u64; 64].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let spec = KernelTask::bootstrap_spec(
            TaskId(2000 + i),
            (i % 8) as u8,
            i as u64,
            (i * 2) as u64,
            top as u64,
            0x9000 + i as u64,
            entry,
        );
        let task = KernelTask::new_shared_from_spec(spec);
        let task = task.lock();
        assert_eq!(task.id, TaskId(2000 + i));
        assert_eq!(task.context.rip, entry);
        assert_eq!(task.context.rsp, task.kernel_stack_pointer);
        assert_eq!(task.page_table_root, 0x9000 + i as u64);
    }
}

#[test]
fn x86_64_new_shared_bootstrap_matches_shared_from_spec_contract() {
    let mut stack = vec![0u64; 32].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let entry = host_test_entry as *const () as usize as u64;

    let direct = KernelTask::new_shared_bootstrap(TaskId(4600), 2, 5, 9, top as u64, 0x5678, entry);
    let spec = KernelTask::new_shared_from_spec(KernelTask::bootstrap_spec(
        TaskId(4600),
        2,
        5,
        9,
        top as u64,
        0x5678,
        entry,
    ));

    let direct = direct.lock();
    let spec = spec.lock();
    assert_eq!(direct.id, spec.id);
    assert_eq!(direct.priority, spec.priority);
    assert_eq!(direct.deadline, spec.deadline);
    assert_eq!(direct.burst_time, spec.burst_time);
    assert_eq!(direct.page_table_root, spec.page_table_root);
    assert_eq!(direct.context.rip, spec.context.rip);
    assert_eq!(direct.context.rsp, spec.context.rsp);
    assert_eq!(direct.kernel_stack_pointer, spec.kernel_stack_pointer);
}

#[test]
fn snapshot_module_image_is_repeatably_stable_for_aligned_probe_image() {
    let image = aligned_probe_linked_elf_bytes();
    let first = snapshot_module_image(&image).expect("first snapshot");

    for _ in 0..8 {
        let snapshot = snapshot_module_image(&image).expect("repeat snapshot");
        assert_eq!(snapshot.info.machine, first.info.machine);
        assert_eq!(snapshot.info.program_headers, first.info.program_headers);
        assert_eq!(snapshot.load_plan.segments.len(), first.load_plan.segments.len());
        assert_eq!(snapshot.mappings.len(), first.mappings.len());
        assert_eq!(snapshot.load_plan.total_file_bytes, first.load_plan.total_file_bytes);
        assert_eq!(snapshot.load_plan.total_mem_bytes, first.load_plan.total_mem_bytes);
    }
}

#[test]
fn snapshot_module_image_matches_inspect_and_load_plan_contract() {
    let image = aligned_probe_linked_elf_bytes();
    let snapshot = snapshot_module_image(&image).expect("snapshot");
    let info = inspect_elf_image(&image).expect("inspect image");
    let plan = build_load_plan(&image).expect("build load plan");
    let mappings = build_virtual_mapping_plan(&image).expect("build mapping plan");

    assert_eq!(snapshot.info.entry, info.entry);
    assert_eq!(snapshot.info.program_headers, info.program_headers);
    assert_eq!(snapshot.info.program_header_addr, info.program_header_addr);
    assert_eq!(snapshot.info.machine, info.machine);
    assert_eq!(
        snapshot
            .load_plan
            .entry
            .saturating_sub(snapshot.load_plan.aslr_base),
        info.entry
    );
    assert_eq!(plan.entry.saturating_sub(plan.aslr_base), info.entry);
    assert_eq!(snapshot.load_plan.segments.len(), plan.segments.len());
    assert_eq!(snapshot.mappings.len(), mappings.len());
}

#[test]
fn snapshot_module_image_repeatedly_matches_inspect_contract() {
    let image = aligned_probe_linked_elf_bytes();

    for _ in 0..8 {
        let snapshot = snapshot_module_image(&image).expect("snapshot");
        let info = inspect_elf_image(&image).expect("inspect image");
        assert_eq!(snapshot.info.entry, info.entry);
        assert_eq!(snapshot.info.program_headers, info.program_headers);
        assert_eq!(snapshot.info.machine, info.machine);
    }
}

#[test]
fn snapshot_module_image_repeatedly_matches_public_plan_and_mapping_shapes() {
    let image = aligned_probe_linked_elf_bytes();

    for _ in 0..8 {
        let snapshot = snapshot_module_image(&image).expect("snapshot");
        let plan = build_load_plan(&image).expect("build load plan");
        let mappings = build_virtual_mapping_plan(&image).expect("build mapping plan");

        assert_eq!(snapshot.load_plan.segments.len(), plan.segments.len());
        assert_eq!(snapshot.load_plan.total_file_bytes, plan.total_file_bytes);
        assert_eq!(snapshot.load_plan.total_mem_bytes, plan.total_mem_bytes);
        assert_eq!(snapshot.mappings.len(), mappings.len());
    }
}

#[test]
fn inspect_elf_image_rejects_too_small_images() {
    let err = inspect_elf_image(&[0u8; 8]).expect_err("tiny image must fail");
    assert_eq!(err, ModuleLoadError::TooSmall);
}

#[test]
fn inspect_elf_image_rejects_unsupported_class_images() {
    let image = unsupported_class_probe_linked_elf_bytes();
    let err = inspect_elf_image(&image).expect_err("unsupported class must fail");
    assert_eq!(err, ModuleLoadError::UnsupportedClass);
}

#[test]
fn prepare_process_image_entry_from_snapshot_repeatedly_matches_snapshot_entry_contract() {
    let image = aligned_probe_linked_elf_bytes();
    let snapshot = snapshot_module_image(&image).expect("snapshot");

    for _ in 0..8usize {
        let process = make_bootstrap_process(b"snapshot-entry");
        let entry = prepare_process_image_entry_from_snapshot(&process, &image, snapshot.clone())
            .expect("prepare entry from snapshot");
        let runtime = process.runtime_contract_snapshot();
        let (state, status, _) = process.runtime_state();
        let (_, image_pages, _, _) = process.image_state();
        let (_, mapped_pages) = process.mapping_state();

        assert_eq!(entry, runtime.runtime_entry as u64);
        assert_ne!(entry, 0);
        assert_eq!(state, ProcessLifecycleState::Runnable);
        assert_eq!(status, 0);
        assert_ne!(image_pages, 0);
        assert_ne!(mapped_pages, 0);
    }
}

#[test]
fn cfs_singleton_bootstrap_path_returns_the_only_task() {
    let mut scheduler = CFS::new();
    scheduler.init();
    scheduler.add_task(make_arc_task(1, 10));

    assert_eq!(scheduler.runqueue_len(), 1);
    assert_eq!(scheduler.bootstrap_pick_next(), Some(TaskId(1)));
    assert_eq!(scheduler.pick_next(), Some(TaskId(1)));
}

#[test]
fn round_robin_cycles_single_task() {
    let mut scheduler = RoundRobin::new();
    scheduler.add_task(make_arc_task(7, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(7)));
    assert_eq!(scheduler.tick(TaskId(7)), SchedulerAction::Reschedule);
}

#[test]
fn weighted_round_robin_returns_enqueued_task() {
    let mut scheduler = WeightedRoundRobin::new();
    scheduler.add_task(make_arc_task(8, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(8)));
}

#[test]
fn fifo_returns_oldest_enqueued_task() {
    let mut scheduler = FIFO::new();
    scheduler.add_task(make_arc_task(9, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(9)));
}

#[test]
fn cooperative_reports_front_task_without_preemption() {
    let mut scheduler = Cooperative::new();
    scheduler.add_task(make_task(10, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(10)));
    assert_eq!(scheduler.tick(TaskId(10)), SchedulerAction::Continue);
}

#[test]
fn lifo_prefers_most_recent_task() {
    let mut scheduler = LIFO::new();
    scheduler.add_task(make_task(11, 10));
    scheduler.add_task(make_task(12, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(12)));
}

#[test]
fn lottery_singleton_is_stable() {
    let mut scheduler = Lottery::new();
    scheduler.add_task(make_task(13, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(13)));
}

#[test]
fn mlfq_singleton_survives_tick_and_pick() {
    let mut scheduler = MLFQ::new();
    scheduler.add_task(make_arc_task(14, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(14)));
    let _ = scheduler.tick(TaskId(14));
}

#[test]
fn edf_singleton_returns_deadline_task() {
    let mut scheduler = EDF::new();
    scheduler.add_task(make_arc_task(15, 10));

    assert_eq!(scheduler.pick_next(), Some(TaskId(15)));
}

#[test]
fn process_runtime_entry_overrides_image_entry_in_snapshot() {
    let process = make_process(b"proc");
    process.image_entry.store(0x1111, core::sync::atomic::Ordering::Relaxed);
    process.set_runtime_entry(Some(0x2222));
    process.set_runtime_fini_entry(Some(0x3333));

    let snapshot = process.runtime_contract_snapshot();
    assert_eq!(snapshot.image_entry, 0x1111);
    assert_eq!(snapshot.runtime_entry, 0x2222);
    assert_eq!(snapshot.runtime_fini_entry, 0x3333);
}

#[test]
fn process_runtime_snapshot_falls_back_to_image_entry_when_runtime_entry_cleared() {
    let process = make_process(b"proc");
    process.image_entry.store(0x4444, core::sync::atomic::Ordering::Relaxed);
    process.set_runtime_entry(Some(0x5555));
    process.set_runtime_entry(None);

    let snapshot = process.runtime_contract_snapshot();
    assert_eq!(snapshot.runtime_entry, 0x4444);
}

#[test]
fn process_lifecycle_markers_transition_cleanly() {
    let process = make_process(b"proc");
    process.mark_runnable();
    let (state, status, _) = process.runtime_state();
    assert_eq!(state, ProcessLifecycleState::Runnable);
    assert_eq!(status, 0);

    process.mark_running();
    let (state, _, _) = process.runtime_state();
    assert_eq!(state, ProcessLifecycleState::Running);

    process.mark_exited(17);
    let (state, status, _) = process.runtime_state();
    assert_eq!(state, ProcessLifecycleState::Exited);
    assert_eq!(status, 17);
}

#[test]
fn process_add_thread_records_membership() {
    let process = make_process(b"proc");
    process.add_thread(TaskId(99));
    process.add_thread(TaskId(100));

    let threads = process.threads.lock();
    assert_eq!(threads.as_slice(), &[TaskId(99), TaskId(100)]);
}

#[test]
fn task_registry_snapshot_tracks_registered_arc_task() {
    let tid = TaskId(50_001);
    let task = Arc::new(IrqSafeMutex::new(make_task(tid.0, 10)));

    register_task_arc(task.clone());

    let mut ids = [TaskId(0); 16];
    let written = task_ids_snapshot(&mut ids);
    assert!(written >= 1);
    assert!(ids[..written].contains(&tid));

    let snapshot = task_context_snapshot(tid).expect("task snapshot");
    assert_eq!(snapshot.0, hypercore::interfaces::TaskState::Ready);
    assert_eq!(snapshot.1, None);
    assert_eq!(snapshot.2, 0);
    assert_ne!(snapshot.3, 0);
    assert!(get_task(tid).is_some());

    unregister_task(tid);
    assert!(get_task(tid).is_none());
}

#[test]
fn task_registry_snapshot_tracks_registered_entry_shape() {
    let tid = TaskId(50_101);
    let task = Arc::new(IrqSafeMutex::new(make_task(tid.0, 10)));
    register_task_arc(task.clone());

    let mut entries = [hypercore::kernel::task::TaskRegistrySnapshotEntry::default(); 8];
    let written = task_registry_snapshot(&mut entries);
    assert!(written >= 1);
    let found = entries[..written]
        .iter()
        .find(|entry| entry.task_id == tid)
        .expect("task registry entry");
    assert_eq!(found.process_id, 0);
    assert_ne!(found.kernel_stack_pointer, 0);

    unregister_task(tid);
}

#[test]
fn register_task_arc_is_repeatably_stable_for_fresh_ids() {
    let _guard = task_registry_test_guard();
    for round in 0..16usize {
        let tid = TaskId(50_200 + round);
        let task = Arc::new(IrqSafeMutex::new(make_task(tid.0, 10)));
        register_task_arc(task);

        let mut entries = [hypercore::kernel::task::TaskRegistrySnapshotEntry::default(); 32];
        let written = task_registry_snapshot(&mut entries);
        let found = entries[..written]
            .iter()
            .find(|entry| entry.task_id == tid)
            .expect("task registry entry");
        assert_eq!(found.task_id, tid);
        assert_ne!(found.kernel_stack_pointer, 0);

        unregister_task(tid);
        assert!(get_task(tid).is_none());
    }
}

#[test]
fn register_task_arc_overwrites_same_id_with_latest_arc() {
    let _guard = task_registry_test_guard();
    let tid = TaskId(50_400);
    let first = Arc::new(IrqSafeMutex::new(make_task(tid.0, 1)));
    let second = Arc::new(IrqSafeMutex::new(make_task(tid.0, 9)));

    register_task_arc(first);
    register_task_arc(second.clone());

    let fetched = get_task(tid).expect("registered task");
    let fetched = fetched.lock();
    assert_eq!(fetched.priority, 9);

    unregister_task(tid);
    assert!(get_task(tid).is_none());
}

#[test]
fn cfs_add_task_preserves_registered_arc_identity() {
    let _guard = task_registry_test_guard();
    let tid = TaskId(50_500);
    let task = Arc::new(IrqSafeMutex::new(make_task(tid.0, 6)));
    let mut scheduler = CFS::new();

    register_task_arc(task.clone());
    scheduler.add_task(task.clone());

    let scheduled = scheduler.get_task_mut(tid).expect("scheduler task");
    assert!(Arc::ptr_eq(&scheduled, &task));

    unregister_task(tid);
    assert!(get_task(tid).is_none());
}

#[test]
fn cfs_add_task_is_repeatably_stable_for_registered_bootstrap_tasks() {
    for round in 0..8usize {
        let tid = TaskId(50_600 + round);
        let task = Arc::new(IrqSafeMutex::new(make_task(tid.0, 4)));
        let mut scheduler = CFS::new();

        register_task_arc(task.clone());
        scheduler.add_task(task.clone());

        let scheduled = scheduler.get_task_mut(tid).expect("scheduler task");
        let scheduled = scheduled.lock();
        assert_eq!(scheduled.id, tid);
        assert_eq!(scheduled.state, hypercore::interfaces::TaskState::Ready);

        unregister_task(tid);
        assert!(get_task(tid).is_none());
    }
}

#[test]
fn launch_registry_snapshot_defaults_to_empty_when_registry_is_empty() {
    let mut entries = [LaunchRegistrySnapshotEntry::default(); 4];
    let written = launch_registry_snapshot(&mut entries);
    assert_eq!(written, 0);
    assert_eq!(entries[0].process_id, hypercore::interfaces::ProcessId(0));
    assert_eq!(entries[0].task_id, TaskId(0));
    assert_eq!(entries[0].stage, 0);
}

#[test]
fn launch_registry_snapshot_stays_empty_after_invalid_static_spawn() {
    let before = process_count();
    let mut entries = [LaunchRegistrySnapshotEntry::default(); 4];

    assert_eq!(
        spawn_bootstrap_from_static_image(b"", b"abc", 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );

    let written = launch_registry_snapshot(&mut entries);
    assert_eq!(written, 0);
    assert_eq!(process_count(), before);
}

#[test]
fn invalid_static_spawn_does_not_leak_task_registry_entries() {
    let mut before = [TaskId(0); 128];
    let before_written = task_ids_snapshot(&mut before);

    assert_eq!(
        spawn_bootstrap_from_static_image(b"", b"abc", 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );

    let mut after = [TaskId(0); 128];
    let after_written = task_ids_snapshot(&mut after);
    assert_eq!(after_written, before_written);
    assert_eq!(&after[..after_written], &before[..before_written]);
}

#[test]
fn invalid_aligned_static_spawn_does_not_leak_task_registry_entries() {
    let mut before = [TaskId(0); 128];
    let before_written = task_ids_snapshot(&mut before);

    assert_eq!(
        spawn_bootstrap_from_aligned_static_image(b"", b"abc", 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );

    let mut after = [TaskId(0); 128];
    let after_written = task_ids_snapshot(&mut after);
    assert_eq!(after_written, before_written);
    assert_eq!(&after[..after_written], &before[..before_written]);
}

#[test]
fn invalid_owned_spawn_does_not_leak_task_registry_entries() {
    let mut before = [TaskId(0); 128];
    let before_written = task_ids_snapshot(&mut before);

    assert_eq!(
        spawn_bootstrap_from_image(b"", &[1u8], 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );

    let mut after = [TaskId(0); 128];
    let after_written = task_ids_snapshot(&mut after);
    assert_eq!(after_written, before_written);
    assert_eq!(&after[..after_written], &before[..before_written]);
}

#[test]
fn debug_trace_recent_records_copy_preserves_order_and_values() {
    let _guard = trace_test_guard();
    let (scope, records, latest_two) = (0..4)
        .find_map(|_| {
            let scope = unique_trace_scope("htr_copy_ord");
            record_with_metadata(
                &scope,
                "one",
                Some(0x11),
                false,
                TraceSeverity::Trace,
                TraceCategory::Launch,
            );
            record_with_metadata(
                &scope,
                "two",
                Some(0x22),
                false,
                TraceSeverity::Warn,
                TraceCategory::Task,
            );

            let records = recent_records_vec_copy();
            let latest_two = records
                .iter()
                .filter(|record| record.scope_str() == scope && record.stage_str() == "two")
                .max_by_key(|record| record.seq)
                .copied();
            latest_two.map(|latest_two| (scope, records, latest_two))
        })
        .expect("latest matching stage two trace");

    assert!(!records.is_empty());
    let matching: Vec<_> = records
        .iter()
        .filter(|record| record.scope_str() == scope)
        .collect();
    assert!(!matching.is_empty());
    assert_eq!(latest_two.value, 0x22);
    if let Some(first) = matching
        .iter()
        .filter(|record| record.stage_str() == "one")
        .max_by_key(|record| record.seq)
    {
        assert_eq!(first.value, 0x11);
        assert!(first.seq <= latest_two.seq);
    }
    record_with_metadata(
        &scope,
        "two",
        Some(0x22),
        false,
        TraceSeverity::Warn,
        TraceCategory::Task,
    );
    let mut task_records = [TraceRecord::EMPTY; 64];
    let task_written = recent_records_for_category(TraceCategory::Task, &mut task_records);
    let matching_task: Vec<_> = task_records[..task_written]
        .iter()
        .filter(|record| record.scope_str() == scope)
        .collect();
    assert!(!matching_task.is_empty());
    let latest_task = matching_task
        .iter()
        .max_by_key(|record| record.seq)
        .expect("latest matching task trace");
    assert_eq!(latest_task.stage_str(), "two");
    assert_eq!(latest_task.value, 0x22);
}

#[test]
fn debug_trace_category_queries_return_latest_matching_record() {
    let _guard = trace_test_guard();
    let mut latest_loader = None;

    for _ in 0..4 {
        let loader_scope = unique_trace_scope("hldr_cat_q");
        let task_scope = unique_trace_scope("htsk_cat_q");
        record_with_metadata(
            &loader_scope,
            "begin",
            Some(0x33),
            false,
            TraceSeverity::Trace,
            TraceCategory::Loader,
        );
        record_with_metadata(
            &loader_scope,
            "returned",
            Some(0x44),
            false,
            TraceSeverity::Trace,
            TraceCategory::Loader,
        );
        record_with_metadata(
            &task_scope,
            "ready",
            Some(0x55),
            false,
            TraceSeverity::Trace,
            TraceCategory::Task,
        );

        let latest = latest_record().expect("latest trace");
        assert!(latest.seq != 0);

        let records = recent_records_vec_copy();
        if let Some(record) = records
            .iter()
            .filter(|record| record.scope_str() == loader_scope && record.stage_str() == "returned")
            .max_by_key(|record| record.seq)
        {
            latest_loader = Some(*record);
            break;
        }
    }

    let latest_loader = latest_loader.expect("latest matching loader trace");
    assert_eq!(latest_loader.stage_str(), "returned");
    assert_eq!(latest_loader.value, 0x44);
}


#[test]
fn process_add_thread_handles_larger_thread_sets() {
    let process = make_process(b"proc");
    for tid in 0..64 {
        process.add_thread(TaskId(tid));
    }

    let threads = process.threads.lock();
    assert_eq!(threads.len(), 64);
    assert_eq!(threads.first().copied(), Some(TaskId(0)));
    assert_eq!(threads.last().copied(), Some(TaskId(63)));
}

#[test]
fn process_add_thread_marks_bootstrap_membership_before_publish() {
    let process = make_process(b"proc");
    assert_eq!(process.lifecycle_state(), ProcessLifecycleState::Created);
    process.add_bootstrap_thread(TaskId(7));
    process.add_bootstrap_thread(TaskId(8));

    let threads = process.threads.lock();
    assert_eq!(threads.as_slice(), &[TaskId(7), TaskId(8)]);
}

#[test]
fn process_new_preallocates_thread_storage() {
    let process = make_process(b"proc");
    let threads = process.threads.lock();
    assert!(threads.capacity() >= 4);
    assert!(threads.is_empty());
}

#[test]
fn process_new_bootstrap_skips_early_thread_preallocation() {
    let process = make_bootstrap_process(b"proc");
    let threads = process.threads.lock();
    assert_eq!(threads.capacity(), 0);
    assert!(threads.is_empty());
}

#[test]
fn process_new_bootstrap_accepts_unpublished_thread_membership() {
    let process = make_bootstrap_process(b"proc");
    process.add_bootstrap_thread(TaskId(71));
    process.add_bootstrap_thread(TaskId(72));

    let threads = process.threads.lock();
    assert_eq!(threads.as_slice(), &[TaskId(71), TaskId(72)]);
}

#[test]
fn process_new_bootstrap_repeated_unpublished_membership_stays_stable() {
    for round in 0..32usize {
        let process = make_bootstrap_process(b"proc");
        for tid in 0..8usize {
            process.add_bootstrap_thread(TaskId(round * 16 + tid));
        }

        let threads = process.threads.lock();
        assert_eq!(threads.len(), 8);
        assert_eq!(threads.first().copied(), Some(TaskId(round * 16)));
        assert_eq!(threads.last().copied(), Some(TaskId(round * 16 + 7)));
    }
}

#[test]
fn process_add_bootstrap_thread_matches_bootstrap_membership_contract() {
    let process = make_bootstrap_process(b"proc");
    process.add_bootstrap_thread(TaskId(501));
    process.add_bootstrap_thread(TaskId(502));

    let threads = process.threads.lock();
    assert_eq!(threads.as_slice(), &[TaskId(501), TaskId(502)]);
}

#[test]
fn process_bind_tls_template_copies_tls_bytes_without_publish_locking() {
    let process = make_process(b"tls");
    let image = [0xAAu8, 0xBB, 0xCC, 0xDD, 0xEE, 0xF0];
    let plan = ModuleLoadPlan {
        entry: 0,
        segments: vec![LoadSegmentPlan {
            virtual_addr: 0x2000,
            file_offset: 1,
            file_size: 5,
            mem_size: 5,
            align: 1,
        }],
        total_file_bytes: 5,
        total_mem_bytes: 5,
        aslr_base: 0,
        tls_virtual_addr: 0x2002,
        tls_file_size: 3,
        tls_mem_size: 8,
        tls_align: 8,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };
    process
        .bind_module_load_plan(&plan)
        .expect("bind tls load plan");

    process
        .bind_tls_template(&image, &plan)
        .expect("bind tls template");

    let (tls, mem_size, align) = process.tls_state_snapshot();
    assert_eq!(tls, vec![0xDD, 0xEE, 0xF0]);
    assert_eq!(mem_size, 8);
    assert_eq!(align, 8);
}

#[test]
fn process_bind_tls_template_replaces_previous_tls_buffer_contents() {
    let process = make_process(b"tls");
    let first = ModuleLoadPlan {
        entry: 0,
        segments: vec![LoadSegmentPlan {
            virtual_addr: 0x2000,
            file_offset: 0,
            file_size: 4,
            mem_size: 4,
            align: 1,
        }],
        total_file_bytes: 4,
        total_mem_bytes: 4,
        aslr_base: 0,
        tls_virtual_addr: 0x2001,
        tls_file_size: 2,
        tls_mem_size: 4,
        tls_align: 4,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };
    process.bind_module_load_plan(&first).expect("bind first plan");
    process
        .bind_tls_template(&[1u8, 2, 3, 4], &first)
        .expect("bind first tls");

    let second = ModuleLoadPlan {
        entry: 0,
        segments: vec![LoadSegmentPlan {
            virtual_addr: 0x4000,
            file_offset: 2,
            file_size: 5,
            mem_size: 6,
            align: 1,
        }],
        total_file_bytes: 5,
        total_mem_bytes: 6,
        aslr_base: 0,
        tls_virtual_addr: 0x4003,
        tls_file_size: 3,
        tls_mem_size: 6,
        tls_align: 8,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };
    process.bind_module_load_plan(&second).expect("bind second plan");
    process
        .bind_tls_template(&[9u8, 8, 7, 6, 5, 4, 3, 2, 1, 0], &second)
        .expect("bind second tls");

    let (tls, mem_size, align) = process.tls_state_snapshot();
    assert_eq!(tls, vec![4, 3, 2]);
    assert_eq!(mem_size, 6);
    assert_eq!(align, 8);
}

#[test]
fn prepare_process_image_entry_publishes_runtime_entry_contract() {
    let image_words = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(image_words.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_bootstrap_process(b"probe-entry");

    let entry_only = prepare_process_image_entry(&process, image).expect("prepare entry only");
    let runtime = process.runtime_contract_snapshot();

    assert_eq!(entry_only, runtime.runtime_entry as u64);
    assert_ne!(entry_only, 0);
}

#[test]
fn prepare_process_image_entry_is_repeatably_stable_for_probe_image() {
    let image_words = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(image_words.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };

    for _ in 0..16 {
        let process = make_bootstrap_process(b"probe-entry");
        let entry = prepare_process_image_entry(&process, image).expect("prepare entry only");
        assert_ne!(entry, 0);
    }
}

#[test]
fn prepare_process_image_entry_repeatedly_updates_same_process_consistently() {
    let image_words = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(image_words.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_bootstrap_process(b"probe-entry-shared");

    for iteration in 0..8 {
        let entry = prepare_process_image_entry(&process, image).expect("prepare entry only");
        let runtime = process.runtime_contract_snapshot();
        let (state, status, generation) = process.runtime_state();
        assert_eq!(entry, runtime.runtime_entry as u64);
        assert_eq!(state, ProcessLifecycleState::Runnable);
        assert_eq!(status, 0);
        assert_eq!(generation, (iteration + 1) as u64);
        assert_ne!(entry, 0);
        assert_eq!(runtime.image_entry as u64, entry);
    }
}

#[test]
fn process_bind_tls_template_empty_plan_clears_previous_tls_bytes() {
    let process = make_process(b"tls");
    let seeded = ModuleLoadPlan {
        entry: 0,
        segments: vec![LoadSegmentPlan {
            virtual_addr: 0x3000,
            file_offset: 0,
            file_size: 4,
            mem_size: 4,
            align: 1,
        }],
        total_file_bytes: 4,
        total_mem_bytes: 4,
        aslr_base: 0,
        tls_virtual_addr: 0x3000,
        tls_file_size: 2,
        tls_mem_size: 2,
        tls_align: 2,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };
    process
        .bind_module_load_plan(&seeded)
        .expect("bind seeded load plan");
    process
        .bind_tls_template(&[1u8, 2, 3, 4], &seeded)
        .expect("seed tls template");

    let empty = ModuleLoadPlan {
        entry: 0,
        segments: vec![],
        total_file_bytes: 0,
        total_mem_bytes: 0,
        aslr_base: 0,
        tls_virtual_addr: 0,
        tls_file_size: 0,
        tls_mem_size: 0,
        tls_align: 1,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };
    process
        .bind_module_load_plan(&empty)
        .expect("bind empty load plan");
    process
        .bind_tls_template(&[], &empty)
        .expect("clear tls template");

    let (tls, mem_size, align) = process.tls_state_snapshot();
    assert!(tls.is_empty());
    assert_eq!(mem_size, 0);
    assert_eq!(align, 1);
}

#[test]
fn bind_prepared_image_snapshot_repeatedly_preserves_empty_tls_contract() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    for _ in 0..6 {
        hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
            .expect("bind prepared snapshot");
        let (tls, mem_size, align) = process.tls_state_snapshot();
        assert!(tls.is_empty());
        assert_eq!(mem_size, snapshot.load_plan.tls_mem_size);
        assert_eq!(align, snapshot.load_plan.tls_align.max(1));
    }
}

#[test]
fn bind_prepared_image_snapshot_clears_seeded_tls_bytes_and_republishes_header() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    let seeded = ModuleLoadPlan {
        entry: 0,
        segments: vec![LoadSegmentPlan {
            virtual_addr: 0x3000,
            file_offset: 0,
            file_size: 4,
            mem_size: 4,
            align: 1,
        }],
        total_file_bytes: 4,
        total_mem_bytes: 4,
        aslr_base: 0,
        tls_virtual_addr: 0x3000,
        tls_file_size: 3,
        tls_mem_size: 5,
        tls_align: 8,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };

    process
        .bind_module_load_plan(&seeded)
        .expect("bind seeded load plan");
    process
        .bind_tls_template(&[9u8, 8, 7, 6], &seeded)
        .expect("seed tls");

    hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
        .expect("bind prepared snapshot");

    let (tls, mem_size, align) = process.tls_state_snapshot();
    assert!(tls.is_empty());
    assert_eq!(mem_size, snapshot.load_plan.tls_mem_size);
    assert_eq!(align, snapshot.load_plan.tls_align.max(1));
}

#[test]
fn bind_prepared_image_snapshot_repeatedly_clears_seeded_tls_state() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    for round in 0..6usize {
        let seeded = ModuleLoadPlan {
            entry: 0,
            segments: vec![LoadSegmentPlan {
                virtual_addr: 0x4000 + (round as u64) * 0x1000,
                file_offset: 0,
                file_size: 4,
                mem_size: 4,
                align: 1,
            }],
            total_file_bytes: 4,
            total_mem_bytes: 4,
            aslr_base: 0,
            tls_virtual_addr: 0x4000 + (round as u64) * 0x1000,
            tls_file_size: 2,
            tls_mem_size: 6,
            tls_align: 16,
            program_header_addr: 0,
            program_header_entry_size: 0,
            program_headers: 0,
        };

        process
            .bind_module_load_plan(&seeded)
            .expect("bind seeded load plan");
        process
            .bind_tls_template(&[1u8, 2, 3, 4], &seeded)
            .expect("seed tls");

        hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
            .expect("bind prepared snapshot");

        let (tls, mem_size, align) = process.tls_state_snapshot();
        assert!(tls.is_empty());
        assert_eq!(mem_size, snapshot.load_plan.tls_mem_size);
        assert_eq!(align, snapshot.load_plan.tls_align.max(1));
    }
}

#[test]
fn bind_prepared_image_snapshot_recovers_from_combined_dirty_mapping_and_tls_state() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    let seeded = ModuleLoadPlan {
        entry: 0,
        segments: vec![LoadSegmentPlan {
            virtual_addr: 0x5000,
            file_offset: 0,
            file_size: 4,
            mem_size: 4,
            align: 1,
        }],
        total_file_bytes: 4,
        total_mem_bytes: 4,
        aslr_base: 0,
        tls_virtual_addr: 0x5000,
        tls_file_size: 2,
        tls_mem_size: 8,
        tls_align: 8,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };

    for i in 0..6usize {
        process
            .mapped_regions
            .store(i * 19 + 7, core::sync::atomic::Ordering::Relaxed);
        process
            .mapped_pages
            .store(i * 23 + 11, core::sync::atomic::Ordering::Relaxed);
        process
            .bind_module_load_plan(&seeded)
            .expect("bind seeded load plan");
        process
            .bind_tls_template(&[7u8, 6, 5, 4], &seeded)
            .expect("seed tls");

        hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
            .expect("bind prepared snapshot");

        let runtime = process.runtime_contract_snapshot();
        let (regions, pages) = process.mapping_state();
        let (tls, mem_size, align) = process.tls_state_snapshot();

        assert_eq!(runtime.runtime_entry, snapshot.load_plan.entry as usize);
        assert_eq!(regions, snapshot.mappings.len());
        assert_eq!(pages, snapshot.mappings.iter().fold(0usize, |acc, mapping| {
            acc + (((mapping.end - mapping.start)
                / hypercore::interfaces::memory::PAGE_SIZE_4K as u64) as usize)
        }));
        assert!(tls.is_empty());
        assert_eq!(mem_size, snapshot.load_plan.tls_mem_size);
        assert_eq!(align, snapshot.load_plan.tls_align.max(1));
    }
}

#[test]
fn launch_spawn_bootstrap_rejects_invalid_owned_requests() {
    assert_eq!(
        spawn_bootstrap_from_image(b"", &[1u8], 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(
        spawn_bootstrap_from_image(b"probe", &[], 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test]
fn launch_spawn_bootstrap_rejects_invalid_static_requests() {
    assert_eq!(
        spawn_bootstrap_from_static_image(b"", b"abc", 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(
        spawn_bootstrap_from_static_image(b"probe", b"", 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test]
fn launch_spawn_bootstrap_rejects_invalid_aligned_static_requests() {
    assert_eq!(
        spawn_bootstrap_from_aligned_static_image(b"", b"abc", 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(
        spawn_bootstrap_from_aligned_static_image(b"probe", b"", 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
}

#[test]
fn create_bootstrap_task_from_aligned_probe_snapshot_preserves_contract() {
    let storage = Box::leak(aligned_probe_linked_elf());
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let mut stack = vec![0u64; 96].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
    let tid = TaskId(0xA11C);

    let (process, task) = Process::create_bootstrap_task_from_snapshot(
        b"probe-aligned-static-ok",
        image,
        snapshot,
        tid,
        7,
        11,
        13,
        top as u64,
    )
    .expect("aligned create from snapshot");

    assert_bootstrap_task_contract(&process, &task, tid);
}

#[test]
fn repeated_invalid_static_spawns_leave_launch_registry_empty() {
    let before = process_count();
    let mut entries = [LaunchRegistrySnapshotEntry::default(); 8];

    for _ in 0..12 {
        assert_eq!(
            spawn_bootstrap_from_static_image(b"", b"abc", 0, 0, 0, 0),
            Err(LaunchError::InvalidSpawnRequest)
        );
        let written = launch_registry_snapshot(&mut entries);
        assert_eq!(written, 0);
    }

    assert_eq!(process_count(), before);
}

#[test]
fn repeated_invalid_aligned_static_spawns_leave_launch_registry_empty() {
    let before = process_count();
    let mut entries = [LaunchRegistrySnapshotEntry::default(); 8];

    for _ in 0..12 {
        assert_eq!(
            spawn_bootstrap_from_aligned_static_image(b"", b"abc", 0, 0, 0, 0),
            Err(LaunchError::InvalidSpawnRequest)
        );
        let written = launch_registry_snapshot(&mut entries);
        assert_eq!(written, 0);
    }

    assert_eq!(process_count(), before);
}

#[test]
fn repeated_invalid_owned_spawns_leave_launch_registry_empty() {
    let before = process_count();
    let mut entries = [LaunchRegistrySnapshotEntry::default(); 8];

    for _ in 0..12 {
        assert_eq!(
            spawn_bootstrap_from_image(b"", &[1u8], 0, 0, 0, 0),
            Err(LaunchError::InvalidSpawnRequest)
        );
        let written = launch_registry_snapshot(&mut entries);
        assert_eq!(written, 0);
    }

    assert_eq!(process_count(), before);
}

#[test]
fn launch_clone_rejects_unknown_process_id_without_mutating_registry() {
    let before = process_count();
    assert_eq!(
        clone_process_from_registered_image(hypercore::interfaces::task::ProcessId(usize::MAX), 0, 0, 0, 0),
        Err(LaunchError::InvalidSpawnRequest)
    );
    assert_eq!(process_count(), before);
    assert!(process_boot_image(usize::MAX).is_none());
}

#[test]
fn probe_linked_elf_loader_plans_are_valid() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let preflight = preflight_module_image(image).expect("probe preflight");
    let info = inspect_elf_image(image).expect("probe inspect");
    let plan = build_load_plan(image).expect("probe load plan");
    let mappings = build_virtual_mapping_plan(image).expect("probe mapping plan");

    assert!(preflight.load_segments >= 1);
    assert!(info.entry != 0);
    assert!(plan.entry != 0);
    assert!(!plan.segments.is_empty());
    assert!(!mappings.is_empty());
}

#[test]
fn probe_linked_elf_single_pass_snapshot_matches_split_loader_views() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };

    let snapshot = snapshot_module_image(image).expect("probe snapshot");
    let info = inspect_elf_image(image).expect("probe inspect");
    let plan = build_load_plan(image).expect("probe load plan");
    let mappings = build_virtual_mapping_plan(image).expect("probe mapping plan");

    assert_eq!(snapshot.info.entry, info.entry);
    assert_eq!(snapshot.info.program_headers, info.program_headers);
    assert_eq!(snapshot.info.program_header_entry_size, info.program_header_entry_size);
    assert_eq!(snapshot.info.section_headers, info.section_headers);
    assert_eq!(snapshot.load_plan.program_headers, plan.program_headers);
    assert_eq!(snapshot.load_plan.program_header_entry_size, plan.program_header_entry_size);
    assert_eq!(snapshot.load_plan.segments.len(), plan.segments.len());
    assert_eq!(snapshot.load_plan.total_file_bytes, plan.total_file_bytes);
    assert_eq!(snapshot.load_plan.total_mem_bytes, plan.total_mem_bytes);
    assert_eq!(snapshot.mappings.len(), mappings.len());
}

#[test]
fn prepare_process_image_binds_probe_runtime_state() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_process(b"probe");
    let prepared = prepare_process_image(&process, image).expect("prepare probe image");

    let snapshot = process.runtime_contract_snapshot();
    let (mapped_regions, mapped_pages) = process.mapping_state();

    assert_eq!(snapshot.image_entry, prepared.load_plan.entry as usize);
    assert_eq!(snapshot.runtime_entry, prepared.load_plan.entry as usize);
    assert_eq!(mapped_regions, prepared.mappings.len());
    assert!(mapped_pages > 0);
    assert!(prepared.info.entry != 0);
}

#[test]
fn prepare_process_image_is_repeatably_stable_for_probe_image() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_process(b"probe");

    for generation in 1..=6u64 {
        let prepared = prepare_process_image(&process, image).expect("prepare probe image");
        let runtime = process.runtime_contract_snapshot();
        let (state, status, seen_generation) = process.runtime_state();
        let (regions, pages) = process.mapping_state();

        assert_eq!(state, ProcessLifecycleState::Runnable);
        assert_eq!(status, 0);
        assert_eq!(seen_generation, generation);
        assert_eq!(runtime.runtime_entry as u64, prepared.load_plan.entry);
        assert_eq!(regions, prepared.mappings.len());
        assert!(pages > 0);
    }
}

#[test]
fn single_pass_snapshot_binding_matches_prepare_process_image_state() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
        .expect("bind prepared snapshot");

    let runtime = process.runtime_contract_snapshot();
    let (regions, pages) = process.mapping_state();
    let (tls, mem_size, align) = process.tls_state_snapshot();

    assert_eq!(runtime.runtime_entry, snapshot.load_plan.entry as usize);
    assert_eq!(regions, snapshot.mappings.len());
    assert!(pages > 0);
    assert!(tls.is_empty());
    assert_eq!(mem_size, snapshot.load_plan.tls_mem_size);
    assert_eq!(align, snapshot.load_plan.tls_align.max(1));
}

#[test]
fn single_pass_snapshot_binding_repeatedly_preserves_runtime_contract_shape() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    for generation in 1..=8u64 {
        hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
            .expect("bind prepared snapshot");

        let runtime = process.runtime_contract_snapshot();
        let (state, status, seen_generation) = process.runtime_state();
        let (regions, pages) = process.mapping_state();

        assert_eq!(state, ProcessLifecycleState::Runnable);
        assert_eq!(status, 0);
        assert_eq!(seen_generation, generation);
        assert_eq!(runtime.runtime_entry, snapshot.load_plan.entry as usize);
        assert_eq!(regions, snapshot.mappings.len());
        assert!(pages > 0);
    }
}

#[test]
fn single_pass_snapshot_binding_repeatedly_overwrites_mapping_counts_without_accumulating() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    for _ in 0..8 {
        hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
            .expect("bind prepared snapshot");

        let (regions, pages) = process.mapping_state();
        assert_eq!(regions, snapshot.mappings.len());
        assert_eq!(pages, snapshot.mappings.iter().fold(0usize, |acc, mapping| {
            acc + (((mapping.end - mapping.start)
                / hypercore::interfaces::memory::PAGE_SIZE_4K as u64) as usize)
        }));
    }
}

#[test]
fn single_pass_snapshot_binding_overwrites_dirty_mapping_state_consistently() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    process.mapped_regions.store(777, core::sync::atomic::Ordering::Relaxed);
    process.mapped_pages.store(999, core::sync::atomic::Ordering::Relaxed);

    hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
        .expect("bind prepared snapshot");

    let (regions, pages) = process.mapping_state();
    assert_eq!(regions, snapshot.mappings.len());
    assert_eq!(pages, snapshot.mappings.iter().fold(0usize, |acc, mapping| {
        acc + (((mapping.end - mapping.start)
            / hypercore::interfaces::memory::PAGE_SIZE_4K as u64) as usize)
    }));
}

#[test]
fn single_pass_snapshot_binding_overwrites_extreme_mapping_state_consistently() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    process
        .mapped_regions
        .store(usize::MAX / 4, core::sync::atomic::Ordering::Relaxed);
    process
        .mapped_pages
        .store(usize::MAX / 8, core::sync::atomic::Ordering::Relaxed);

    for _ in 0..4 {
        hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
            .expect("bind prepared snapshot");
        let (regions, pages) = process.mapping_state();
        assert_eq!(regions, snapshot.mappings.len());
        assert_eq!(pages, snapshot.mappings.iter().fold(0usize, |acc, mapping| {
            acc + (((mapping.end - mapping.start)
                / hypercore::interfaces::memory::PAGE_SIZE_4K as u64) as usize)
        }));
    }
}

#[test]
fn single_pass_snapshot_binding_recovers_after_manual_mapping_counter_churn() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_process(b"probe");

    for i in 0..6usize {
        process
            .mapped_regions
            .store(i * 17 + 3, core::sync::atomic::Ordering::Relaxed);
        process
            .mapped_pages
            .store(i * 31 + 9, core::sync::atomic::Ordering::Relaxed);

        hypercore::kernel::process::bind_prepared_image_snapshot(&process, image, &snapshot)
            .expect("bind prepared snapshot");
        let (regions, pages) = process.mapping_state();
        assert_eq!(regions, snapshot.mappings.len());
        assert_eq!(pages, snapshot.mappings.iter().fold(0usize, |acc, mapping| {
            acc + (((mapping.end - mapping.start)
                / hypercore::interfaces::memory::PAGE_SIZE_4K as u64) as usize)
        }));
    }
}

#[test]
fn bind_module_load_plan_sets_runtime_snapshot_and_marks_runnable() {
    let process = make_process(b"probe");
    let plan = ModuleLoadPlan {
        entry: 0x401000,
        segments: vec![
            LoadSegmentPlan {
                virtual_addr: 0x400000,
                file_offset: 0,
                file_size: 0x1800,
                mem_size: 0x2000,
                align: 0x1000,
            },
            LoadSegmentPlan {
                virtual_addr: 0x404000,
                file_offset: 0x2000,
                file_size: 0x400,
                mem_size: 0x1000,
                align: 0x1000,
            },
        ],
        total_file_bytes: 0x1c00,
        total_mem_bytes: 0x3000,
        aslr_base: 0x200000,
        tls_virtual_addr: 0,
        tls_file_size: 0,
        tls_mem_size: 0,
        tls_align: 1,
        program_header_addr: 0x200040,
        program_header_entry_size: 56,
        program_headers: 9,
    };

    process
        .bind_module_load_plan(&plan)
        .expect("bind module load plan");

    let snapshot = process.runtime_contract_snapshot();
    let (state, status, generation) = process.runtime_state();
    assert_eq!(state, ProcessLifecycleState::Runnable);
    assert_eq!(status, 0);
    assert_eq!(generation, 1);
    assert_eq!(snapshot.image_entry, 0x401000);
    assert_eq!(snapshot.runtime_entry, 0x401000);
    assert_eq!(process.image_pages.load(core::sync::atomic::Ordering::Relaxed), 3);
    assert_eq!(process.image_segments.load(core::sync::atomic::Ordering::Relaxed), 2);
    assert_eq!(process.image_base.load(core::sync::atomic::Ordering::Relaxed), 0x200000);
}

#[test]
fn bind_virtual_mappings_updates_region_and_page_counts() {
    let process = make_process(b"probe");
    process
        .bind_virtual_mappings(&[
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x400000,
                end: 0x402000,
                file_bytes: 0x1800,
                zero_fill_bytes: 0x800,
            },
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x500000,
                end: 0x501000,
                file_bytes: 0x400,
                zero_fill_bytes: 0xc00,
            },
        ])
        .expect("bind virtual mappings");

    let (regions, pages) = process.mapping_state();
    assert_eq!(regions, 2);
    assert_eq!(pages, 3);
}

#[test]
fn bind_virtual_mappings_overwrites_previous_counts_with_new_layout() {
    let process = make_process(b"probe");
    process
        .bind_virtual_mappings(&[
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x400000,
                end: 0x402000,
                file_bytes: 0x1800,
                zero_fill_bytes: 0x800,
            },
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x500000,
                end: 0x501000,
                file_bytes: 0x400,
                zero_fill_bytes: 0xc00,
            },
        ])
        .expect("first bind");
    process
        .bind_virtual_mappings(&[hypercore::kernel::module_loader::VirtualMappingPlan {
            start: 0x700000,
            end: 0x704000,
            file_bytes: 0x1000,
            zero_fill_bytes: 0x3000,
        }])
        .expect("second bind");

    let (regions, pages) = process.mapping_state();
    assert_eq!(regions, 1);
    assert_eq!(pages, 4);
}

#[test]
fn bind_virtual_mappings_rejects_unaligned_ranges() {
    let process = make_process(b"probe");
    let err = process
        .bind_virtual_mappings(&[hypercore::kernel::module_loader::VirtualMappingPlan {
            start: 0x400001,
            end: 0x402000,
            file_bytes: 0x1000,
            zero_fill_bytes: 0x1000,
        }])
        .expect_err("unaligned mapping should fail");
    assert_eq!(err, "unaligned mapping range");
}

#[test]
fn bind_virtual_mappings_rejects_empty_ranges() {
    let process = make_process(b"probe");
    let err = process
        .bind_virtual_mappings(&[hypercore::kernel::module_loader::VirtualMappingPlan {
            start: 0x400000,
            end: 0x400000,
            file_bytes: 0,
            zero_fill_bytes: 0,
        }])
        .expect_err("empty mapping should fail");
    assert_eq!(err, "invalid mapping range");
}

#[test]
fn bind_virtual_mappings_is_repeatably_stable_for_same_layout() {
    let layouts = [
        (
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x400000,
                end: 0x402000,
                file_bytes: 0x1800,
                zero_fill_bytes: 0x800,
            },
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x500000,
                end: 0x501000,
                file_bytes: 0x400,
                zero_fill_bytes: 0xc00,
            },
        ),
        (
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x600000,
                end: 0x603000,
                file_bytes: 0x1000,
                zero_fill_bytes: 0x2000,
            },
            hypercore::kernel::module_loader::VirtualMappingPlan {
                start: 0x700000,
                end: 0x701000,
                file_bytes: 0x1000,
                zero_fill_bytes: 0,
            },
        ),
    ];

    for (idx, layout) in layouts.into_iter().enumerate() {
        let process = make_process(b"probe");
        process
            .bind_virtual_mappings(&[layout.0, layout.1])
            .expect("bind virtual mappings");
        let (regions, pages) = process.mapping_state();
        assert_eq!(regions, 2, "layout {}", idx);
        assert!(pages >= 2, "layout {}", idx);
    }
}

#[test]
fn create_bootstrap_task_from_probe_image_binds_process_and_thread_state() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let mut stack = vec![0u64; 64].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();

    let (process, task) = Process::create_bootstrap_task_from_image(
        b"probe",
        image,
        TaskId(501),
        10,
        0,
        0,
        top as u64,
    )
    .expect("create bootstrap task");

    assert_bootstrap_task_contract(&process, &task, TaskId(501));
    let task = task.lock();
    assert_eq!(task.state, hypercore::interfaces::TaskState::Ready);
}

#[test]
fn create_bootstrap_task_from_probe_image_is_repeatably_stable() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };

    for i in 0..8usize {
        let mut stack = vec![0u64; 96].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();

        let (process, task) = Process::create_bootstrap_task_from_image(
            b"probe",
            image,
            TaskId(700 + i),
            10,
            i as u64,
            (i * 3) as u64,
            top as u64,
        )
        .expect("create bootstrap task");

        assert_bootstrap_task_contract(&process, &task, TaskId(700 + i));
    }
}

#[test]
fn build_process_bootstrap_task_binds_runtime_entry_and_membership() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_bootstrap_process(b"probe-build");
    let mut stack = vec![0u64; 64].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();

    let task = build_process_bootstrap_task(&process, image, TaskId(1501), 7, 11, 13, top as u64)
        .expect("build bootstrap task");

    assert_bootstrap_task_contract(&process, &task, TaskId(1501));
}

#[test]
fn build_process_bootstrap_task_is_repeatably_stable_for_fresh_processes() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };

    for round in 0..8usize {
        let process = make_bootstrap_process(b"probe-build");
        let mut stack = vec![0u64; 96].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();

        let task = build_process_bootstrap_task(
            &process,
            image,
            TaskId(1600 + round),
            9,
            round as u64,
            (round * 5) as u64,
            top as u64,
        )
        .expect("build bootstrap task");

        assert_bootstrap_task_contract(&process, &task, TaskId(1600 + round));
    }
}

#[test]
fn build_process_bootstrap_task_repeatedly_updates_same_process_consistently() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_bootstrap_process(b"probe-build-repeat");

    for round in 0..4usize {
        let mut stack = vec![0u64; 96].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let task_id = TaskId(1700 + round);

        let task = build_process_bootstrap_task(
            &process,
            image,
            task_id,
            8,
            round as u64,
            (round * 7) as u64,
            top as u64,
        )
        .expect("build bootstrap task");

        assert_bootstrap_task_contract(&process, &task, task_id);
        let threads = process.threads.lock();
        assert_eq!(threads.len(), round + 1);
        assert_eq!(threads.as_slice()[round], task_id);
        drop(threads);
    }
}

#[test]
fn build_process_bootstrap_task_after_repeated_entry_prepare_keeps_contract_intact() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_bootstrap_process(b"probe");

    for round in 0..4usize {
        let prepared_entry =
            prepare_process_image_entry(&process, image).expect("prepare entry only");
        let mut stack = vec![0u64; 64].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let tid = TaskId(1700 + round);
        let task = build_process_bootstrap_task(&process, image, tid, 5, 7, 9, top as u64)
            .expect("build bootstrap task");

        assert_bootstrap_task_contract(&process, &task, tid);
        let runtime = process.runtime_contract_snapshot();
        assert_ne!(prepared_entry, 0);
        assert_eq!(task.lock().context.rip, runtime.runtime_entry as u64);
        Box::leak(stack);
    }
}

#[test]
fn create_bootstrap_task_from_image_keeps_process_arc_and_task_contract() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let mut stack = vec![0u64; 96].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();

    let (process, task) = Process::create_bootstrap_task_from_image(
        b"proc-create",
        image,
        TaskId(1750),
        4,
        6,
        8,
        top as u64,
    )
    .expect("create bootstrap task from image");

    assert_bootstrap_task_contract(&process, &task, TaskId(1750));
}

#[test]
fn create_bootstrap_task_from_snapshot_is_repeatably_stable_for_fresh_processes() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };

    for round in 0..6usize {
        let snapshot = snapshot_module_image(image).expect("snapshot");
        let mut stack = vec![0u64; 96].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let tid = TaskId(1751 + round);

        let (process, task) = Process::create_bootstrap_task_from_snapshot(
            b"proc-create-snapshot",
            image,
            snapshot,
            tid,
            4,
            round as u64,
            (round * 5) as u64,
            top as u64,
        )
        .expect("create bootstrap task from snapshot");

        assert_bootstrap_task_contract(&process, &task, tid);
    }
}

#[test]
fn build_process_bootstrap_task_from_snapshot_matches_runtime_contract() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let snapshot = snapshot_module_image(image).expect("snapshot");
    let process = make_bootstrap_process(b"snapshot-build");
    let mut stack = vec![0u64; 96].into_boxed_slice();
    let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();

    let task = build_process_bootstrap_task_from_snapshot(
        &process,
        image,
        snapshot,
        TaskId(1760),
        4,
        6,
        8,
        top as u64,
    )
    .expect("build bootstrap task from snapshot");

    assert_bootstrap_task_contract(&process, &task, TaskId(1760));
}

#[test]
fn build_process_bootstrap_task_from_snapshot_is_repeatably_stable_for_fresh_processes() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };

    for round in 0..6usize {
        let snapshot = snapshot_module_image(image).expect("snapshot");
        let process = make_bootstrap_process(b"snapshot-build");
        let mut stack = vec![0u64; 96].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let tid = TaskId(1761 + round);

        let task = build_process_bootstrap_task_from_snapshot(
            &process,
            image,
            snapshot,
            tid,
            4,
            round as u64,
            (round * 3) as u64,
            top as u64,
        )
        .expect("build bootstrap task from snapshot");

        assert_bootstrap_task_contract(&process, &task, tid);
    }
}

#[test]
fn prepare_entry_then_build_bootstrap_task_stays_consistent_on_same_process() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };
    let process = make_bootstrap_process(b"probe-build-entry");

    for round in 0..4usize {
        let entry = prepare_process_image_entry(&process, image).expect("prepare entry only");
        let mut stack = vec![0u64; 96].into_boxed_slice();
        let top = stack.as_mut_ptr() as usize + stack.len() * core::mem::size_of::<u64>();
        let task_id = TaskId(1800 + round);
        let task = build_process_bootstrap_task(
            &process,
            image,
            task_id,
            5,
            round as u64,
            (round * 9) as u64,
            top as u64,
        )
        .expect("build bootstrap task");

        assert_bootstrap_task_contract(&process, &task, task_id);
        let runtime = process.runtime_contract_snapshot();
        assert_eq!(runtime.runtime_entry as u64, task.lock().context.rip);
        assert_ne!(entry, 0);
    }
}

#[test]
fn bind_tls_template_shortcuts_cleanly_when_tls_is_absent() {
    let process = make_process(b"no-tls");
    let plan = ModuleLoadPlan {
        entry: 0x1000,
        segments: vec![],
        total_file_bytes: 0,
        total_mem_bytes: 0,
        aslr_base: 0,
        tls_virtual_addr: 0,
        tls_file_size: 0,
        tls_mem_size: 0,
        tls_align: 1,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };

    process
        .bind_tls_template(&[], &plan)
        .expect("tls-less bind should succeed");
    assert!(process.tls_template.lock().is_empty());
}

#[test]
fn bind_tls_template_empty_shortcut_is_repeatably_stable() {
    let process = make_process(b"no-tls");
    let plan = ModuleLoadPlan {
        entry: 0x1000,
        segments: vec![],
        total_file_bytes: 0,
        total_mem_bytes: 0,
        aslr_base: 0,
        tls_virtual_addr: 0,
        tls_file_size: 0,
        tls_mem_size: 0,
        tls_align: 1,
        program_header_addr: 0,
        program_header_entry_size: 0,
        program_headers: 0,
    };

    for _ in 0..32 {
        process
            .bind_tls_template(&[], &plan)
            .expect("tls-less bind should stay stable");
        let (tls, mem_size, align) = process.tls_state_snapshot();
        assert!(tls.is_empty());
        assert_eq!(mem_size, 0);
        assert_eq!(align, 1);
    }
}

#[test]
fn cfs_new_starts_empty_and_pick_next_returns_none() {
    let mut scheduler = CFS::new();
    assert!(scheduler.pick_next().is_none());
}

#[test]
fn cfs_new_is_repeatably_stable_and_empty() {
    for _ in 0..32 {
        let mut scheduler = CFS::new();
        assert!(scheduler.pick_next().is_none());
    }
}

#[test]
fn prepare_process_image_across_fresh_processes_is_repeatably_stable() {
    let storage = aligned_probe_linked_elf();
    let image = unsafe {
        core::slice::from_raw_parts(storage.as_ptr() as *const u8, PROBE_LINKED_ELF.len())
    };

    for i in 0..16usize {
        let process = make_process(b"probe");
        let prepared = prepare_process_image(&process, image).expect("prepare probe image");
        let runtime = process.runtime_contract_snapshot();
        let (state, status, generation) = process.runtime_state();
        let (regions, pages) = process.mapping_state();

        assert_eq!(state, ProcessLifecycleState::Runnable);
        assert_eq!(status, 0);
        assert_eq!(generation, 1, "fresh process generation mismatch at iter {}", i);
        assert_eq!(runtime.runtime_entry as u64, prepared.load_plan.entry);
        assert_eq!(regions, prepared.mappings.len());
        assert!(pages > 0);
    }
}
