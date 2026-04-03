/// Task Management & Signal Delivery — Production Grade
///
/// # Signal delivery design
///
/// Signals are stored as a 64-bit bitmask on each task (`pending_signals`).
/// Delivery happens at two points:
///   1. After every context switch (woken task may have a queued signal).
///   2. At syscall return (the syscall layer calls `check_and_deliver_signals`).
///
/// Delivery modifies the saved `Context` (rip / elr) so the next return-to-user
/// jumps straight into the registered handler.  Only **one** signal is delivered
/// per call; repeated calls drain the queue.
///
/// # Locking discipline
///
/// We use a single consistent locking order to prevent deadlock:
///   TASK_REGISTRY → IrqSafeMutex<KernelTask> → process.signal_handlers
///
/// `check_and_deliver_signals` holds the task lock only long enough to snapshot
/// the bitmasks and process_id, then releases it before acquiring the process lock.
pub use crate::interfaces::task::*;
use crate::interfaces::HardwareAbstraction;

use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;

lazy_static! {
    static ref TASK_REGISTRY: IrqSafeMutex<BTreeMap<TaskId, Arc<IrqSafeMutex<KernelTask>>>> =
        IrqSafeMutex::new(BTreeMap::new());
}

// ── Registry operations ───────────────────────────────────────────────────────

pub fn register_task(task: KernelTask) {
    let id = task.id;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry register begin\n");
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_begin", Some(id.0 as u64));
    TASK_REGISTRY
        .lock()
        .insert(id, Arc::new(IrqSafeMutex::new(task)));
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry register returned\n");
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_returned", Some(id.0 as u64));
}

pub fn register_task_arc(task: Arc<IrqSafeMutex<KernelTask>>) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry register_arc lock begin\n");
    let id = task.lock().id;
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry register_arc lock returned\n");
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_arc_begin", Some(id.0 as u64));
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry register_arc begin\n");
    TASK_REGISTRY.lock().insert(id, task);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry register_arc returned\n");
    crate::kernel::debug_trace::record_kernel_context("task.registry", "register_arc_returned", Some(id.0 as u64));
}

pub fn unregister_task(id: TaskId) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry unregister begin\n");
    crate::kernel::debug_trace::record_kernel_context("task.registry", "unregister_begin", Some(id.0 as u64));
    TASK_REGISTRY.lock().remove(&id);
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    crate::hal::serial::write_raw("[EARLY SERIAL] task registry unregister returned\n");
    crate::kernel::debug_trace::record_kernel_context("task.registry", "unregister_returned", Some(id.0 as u64));
}

pub fn get_task(id: TaskId) -> Option<Arc<IrqSafeMutex<KernelTask>>> {
    TASK_REGISTRY.lock().get(&id).cloned()
}

pub fn set_task_state(id: TaskId, state: TaskState) {
    if let Some(t) = get_task(id) {
        t.lock().state = state;
    }
}

/// Returns (count, running, blocked, ready).
pub fn task_stats() -> (usize, usize, usize, usize) {
    let reg = TASK_REGISTRY.lock();
    let total = reg.len();
    let mut running = 0usize;
    let mut blocked = 0usize;
    let mut ready = 0usize;
    for arc in reg.values() {
        match arc.lock().state {
            TaskState::Running => running += 1,
            TaskState::Blocked => blocked += 1,
            TaskState::Ready => ready += 1,
            TaskState::Terminated => {}
        }
    }
    (total, running, blocked, ready)
}

pub fn task_ids_snapshot(out: &mut [TaskId]) -> usize {
    let reg = TASK_REGISTRY.lock();
    let mut written = 0usize;
    for id in reg.keys() {
        if written >= out.len() {
            break;
        }
        out[written] = *id;
        written += 1;
    }
    written
}

#[derive(Debug, Clone, Copy)]
pub struct TaskRegistrySnapshotEntry {
    pub task_id: TaskId,
    pub state: u8,
    pub process_id: usize,
    pub kernel_stack_pointer: usize,
}

impl Default for TaskRegistrySnapshotEntry {
    fn default() -> Self {
        Self {
            task_id: TaskId(0),
            state: 0,
            process_id: 0,
            kernel_stack_pointer: 0,
        }
    }
}

pub fn task_registry_snapshot(out: &mut [TaskRegistrySnapshotEntry]) -> usize {
    let reg = TASK_REGISTRY.lock();
    let mut written = 0usize;
    for (id, task_arc) in reg.iter() {
        if written >= out.len() {
            break;
        }
        let task = task_arc.lock();
        out[written] = TaskRegistrySnapshotEntry {
            task_id: *id,
            state: task.state as u8,
            process_id: task.process_id.map(|pid| pid.0).unwrap_or(0),
            kernel_stack_pointer: task.kernel_stack_pointer as usize,
        };
        written += 1;
    }
    written
}

pub fn task_context_snapshot(id: TaskId) -> Option<(TaskState, Option<ProcessId>, u64, usize)> {
    let task = get_task(id)?;
    let task = task.lock();
    Some((
        task.state,
        task.process_id,
        task.page_table_root,
        task.kernel_stack_pointer as usize,
    ))
}

// ── Signal delivery ───────────────────────────────────────────────────────────

/// Deliver one pending, unmasked signal whose handler is registered.
///
/// Called at two points in the execution path:
/// * After `HAL::context_switch` in `suspend_current_task` (pre-empted tasks).
/// * At the tail of each syscall that returns to user mode.
///
/// # Delivery mechanism
/// The function rewrites `task.context.rip` (x86_64) or `task.context.elr`
/// (AArch64) to the handler's virtual address.  The current `rip`/`elr` is
/// NOT pushed onto the user stack by the kernel — userspace must set up a
/// `sa_restorer` trampoline that calls `sigreturn(2)` to restore the original
/// context.  (That is a userspace ABI concern, not a kernel concern.)
///
/// # No re-entrancy
/// The pending bit is cleared with `fetch_and` before the context is modified,
/// preventing the same signal from being delivered twice even if an interrupt
/// fires between the load and the store.
pub fn check_and_deliver_signals() {
    // ── 1. Identify current task ─────────────────────────────────────────────
    // SAFETY: we are in kernel context, so CpuLocal is always valid.
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let cur_tid = TaskId(cpu.current_task.load(Ordering::Relaxed));

    // Tid 0 is the idle task; no signal delivery needed.
    if cur_tid.0 == 0 {
        return;
    }

    let Some(task_arc) = get_task(cur_tid) else {
        return;
    };

    // ── 2. Snapshot bitmasks (hold task lock as short as possible) ───────────
    let (pending, signal_mask, pid) = {
        let t = task_arc.lock();
        (t.pending_signals, t.signal_mask, t.process_id)
    };

    let deliverable = pending & !signal_mask;
    if deliverable == 0 {
        return;
    }

    // ── 3. Locate the process and its signal handler table ───────────────────
    let Some(pid) = pid else { return };
    #[cfg(feature = "process_abstraction")]
    let Some(proc) = crate::kernel::process_registry::get_process(pid) else {
        return;
    };

    // ── 4. Walk deliverable signals lowest→highest (POSIX: lower number wins) ─
    #[cfg(feature = "process_abstraction")]
    {
        let handlers = proc.signal_handlers.lock();
        for sig in 1i32..64 {
            let bit = 1u64 << sig;
            if (deliverable & bit) == 0 {
                continue;
            }

            let Some(&handler_vaddr) = handlers.get(&sig) else {
                continue;
            };

            // 4a. Atomically clear the pending bit before modifying the context
            //     to prevent re-delivery if we are interrupted here.
            {
                let mut task = task_arc.lock();
                // Re-check: another CPU might have cleared it between our read and now.
                if (task.pending_signals & bit) == 0 {
                    continue;
                }
                task.pending_signals &= !bit;

                // 4b. Rewrite the instruction pointer.
                #[cfg(target_arch = "x86_64")]
                {
                    task.context.rip = handler_vaddr;
                }
                #[cfg(target_arch = "aarch64")]
                {
                    task.context.elr = handler_vaddr;
                }
            }

            crate::klog_trace!(
                "signal: delivered sig={} handler={:#x} tid={}",
                sig,
                handler_vaddr,
                cur_tid.0
            );
            break; // deliver one signal per call
        }
    }
}

// ── Blocking / waking ─────────────────────────────────────────────────────────

/// Block the current task on `wait_queue` and switch to the next runnable task.
///
/// Uses a careful locking order to avoid ABBA deadlocks:
///   1. IRQs disabled.
///   2. Scheduler lock held (to pick next task atomically).
///   3. Task lock acquired only long enough to read/write state.
///   4. Raw pointer used to cross the context-switch boundary (Arc is on heap
///      and will not move).
///   5. After switch, signal delivery runs before re-enabling IRQs.
pub fn suspend_current_task(wait_queue: &crate::kernel::sync::WaitQueue) {
    use crate::hal::HAL;
    use crate::interfaces::Scheduler;
    use crate::kernel::cpu_local::CpuLocal;

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

        // Mark blocked before removing from runqueue.
        {
            current_arc.lock().state = TaskState::Blocked;
        }
        sched.remove_task(current_tid);
        wait_queue.block_id(current_tid);

        // Pick the next runnable task.
        let next_tid = match sched.pick_next() {
            Some(t) => t,
            None => {
                // No runnable task — undo the block and return.
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

        cpu.current_task.store(next_tid.0, Ordering::Relaxed);

        let next_arc = match get_task(next_tid) {
            Some(a) => a,
            None => {
                // Next task disappeared — bail.
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
        {
            next_arc.lock().state = TaskState::Running;
        }

        // Build raw pointer into the heap-allocated KernelTask.
        // Safety: Arc keeps the allocation alive; the pointer is only
        // dereferenced inside the assembly trampoline which runs before unlock.
        let curr_sp = unsafe {
            let base = Arc::as_ptr(&current_arc) as *mut KernelTask;
            &raw mut (*base).kernel_stack_pointer as *mut usize
        };
        let next_sp = next_arc.lock().kernel_stack_pointer as usize;

        (curr_sp, next_sp)
    }; // scheduler lock released here

    unsafe {
        HAL::context_switch(curr_sp_ptr, next_sp);
    }

    // We resume here when this task is woken.
    check_and_deliver_signals();
    HAL::irq_restore(flags);
}

// ── Wake helpers ──────────────────────────────────────────────────────────────

/// Move a single task from Blocked → Ready and re-enqueue it.
///
/// Uses a balanced load-selection: prefers the CPU with the shortest runqueue.
pub fn wake_task(id: TaskId) {
    let Some(task_arc) = get_task(id) else { return };
    {
        let mut task = task_arc.lock();
        if task.state != TaskState::Blocked {
            return;
        }
        task.state = TaskState::Ready;
    }

    // Prefer the CPU that the task last ran on (cache warmth).
    use crate::interfaces::Scheduler;
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

/// Wake a batch of tasks atomically (avoids repeated lock acquisitions).
pub fn wake_tasks(ids: alloc::vec::Vec<TaskId>) {
    // Collect all valid arcs first (one registry lock).
    let arcs: alloc::vec::Vec<Arc<IrqSafeMutex<KernelTask>>> = {
        let reg = TASK_REGISTRY.lock();
        ids.iter().filter_map(|id| reg.get(id).cloned()).collect()
    };

    if arcs.is_empty() {
        return;
    }

    // Transition state.
    for arc in &arcs {
        let mut t = arc.lock();
        if t.state == TaskState::Blocked {
            t.state = TaskState::Ready;
        }
    }

    // Distribute across CPUs using round-robin with runqueue awareness.
    use crate::interfaces::Scheduler;
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
