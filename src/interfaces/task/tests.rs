use super::{CpuId, KernelTask, TaskId};

#[test_case]
fn cpu_id_typed_affinity_helpers_work() {
    let task = KernelTask::new(TaskId(1), 10, 0, 0, 0x1000, 0x2000, 0x3000)
        .with_affinity_mask(1u64 << 3)
        .with_preferred_cpu_id(CpuId(3));

    assert!(task.can_run_on_cpu_id(CpuId(3)));
    assert!(!task.can_run_on_cpu_id(CpuId(2)));
    assert_eq!(task.preferred_cpu, CpuId(3));
    assert!(!task.preferred_cpu.is_any());
}
