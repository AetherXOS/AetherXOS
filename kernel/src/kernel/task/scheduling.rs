use super::*;
use crate::hal::HAL;
use crate::interfaces::Scheduler;
use crate::kernel::cpu_local::CpuLocal;
use core::sync::atomic::Ordering;
use alloc::sync::Arc;

pub fn suspend_current_task(wait_queue: &crate::kernel::sync::WaitQueue) {
    suspend_current_task_with_mask(wait_queue, 0xFFFF_FFFF);
}

pub fn suspend_current_task_with_mask(wait_queue: &crate::kernel::sync::WaitQueue, mask: u32) {
    let flags = HAL::irq_save();
    let cpu = unsafe { CpuLocal::get() };
    let current_tid = TaskId(cpu.current_task.load(Ordering::Relaxed));

    let current_arc = match get_task(current_tid) {
        Some(a) => a,
        None => {
            HAL::irq_restore(flags);
            return;
        }
    };

    let (curr_sp_ptr, next_sp) = {
        let mut sched = cpu.scheduler.lock();

        {
            current_arc.lock().state = TaskState::Blocked;
        }
        sched.remove_task(current_tid);
        wait_queue.block_id_with_mask(current_tid, mask);

        let next_tid = match sched.pick_next() {
            Some(t) => t,
            None => {
                // Work-Stealing: Try to steal from other CPUs
                let mut stolen = None;
                if let Some(cpus) = crate::hal::smp::CPUS.try_lock() {
                    for other_cpu in cpus.iter() {
                        if let Some(mut other_sched) = other_cpu.scheduler.try_lock() {
                            if let Some(task_arc) = other_sched.steal_task() {
                                let tid = task_arc.lock().id;
                                sched.add_task(task_arc);
                                stolen = Some(tid);
                                break;
                            }
                        }
                    }
                }

                if let Some(tid) = stolen {
                    tid
                } else {
                    {
                        current_arc.lock().state = TaskState::Ready;
                    }
                    wait_queue.unblock_id(current_tid);
                    sched.add_task(current_arc.clone());
                    drop(sched);
                    HAL::irq_restore(flags);
                    return;
                }
            }
        };

        cpu.current_task.store(next_tid.0, Ordering::Relaxed);

        let next_arc = match get_task(next_tid) {
            Some(a) => a,
            None => {
                cpu.current_task.store(current_tid.0, Ordering::Relaxed);
                {
                    current_arc.lock().state = TaskState::Ready;
                }
                wait_queue.unblock_id(current_tid);
                sched.add_task(current_arc.clone());
                drop(sched);
                HAL::irq_restore(flags);
                return;
            }
        };
        // Telemetry hook
        cpu.on_context_switch(current_tid, next_tid);

        {
            let mut next_task = next_arc.lock();
            next_task.state = TaskState::Running;
        }

        let curr_sp = unsafe {
            let base = Arc::as_ptr(&current_arc) as *mut KernelTask;
            &raw mut (*base).kernel_stack_pointer as *mut usize
        };
        let next_sp = next_arc.lock().kernel_stack_pointer as usize;

        (curr_sp, next_sp)
    };

    unsafe {
        HAL::context_switch(curr_sp_ptr, next_sp);
    }

    check_and_deliver_signals();
    HAL::irq_restore(flags);
}

pub fn wake_task(id: TaskId) {
    let Some(task_arc) = get_task(id) else { return };
    {
        let mut task = task_arc.lock();
        if task.state != TaskState::Blocked {
            return;
        }
        task.state = TaskState::Ready;
    }

    let cpus = crate::hal::smp::CPUS.lock();
    let cpu_count = cpus.len();
    if cpu_count == 0 {
        return;
    }

    let mut best = 0usize;
    let mut depth = usize::MAX;
    for (i, cpu) in cpus.iter().enumerate() {
        let d = cpu.scheduler.lock().runqueue_len();
        if d < depth {
            depth = d;
            best = i;
        }
    }

    if let Some(cpu) = cpus.get(best) {
        cpu.scheduler.lock().add_task(task_arc);
    }
}

pub fn wake_tasks(ids: alloc::vec::Vec<TaskId>) {
    let arcs: alloc::vec::Vec<Arc<IrqSafeMutex<crate::interfaces::task::KernelTask>>> = {
        let reg = super::registry::TASK_REGISTRY.lock();
        ids.iter().filter_map(|id| reg.get(id).cloned()).collect()
    };

    if arcs.is_empty() {
        return;
    }

    for arc in &arcs {
        let mut t = arc.lock();
        if t.state == TaskState::Blocked {
            t.state = TaskState::Ready;
        }
    }

    let cpus = crate::hal::smp::CPUS.lock();
    let cpu_count = cpus.len();
    if cpu_count == 0 {
        return;
    }

    for (idx, arc) in arcs.into_iter().enumerate() {
        let cpu_idx = idx % cpu_count;
        if let Some(cpu) = cpus.get(cpu_idx) {
            cpu.scheduler.lock().add_task(arc);
        }
    }
}
