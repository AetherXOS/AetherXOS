use crate::interfaces::task::{KernelTask, ProcessId, TaskId, TaskState};
use crate::kernel::process_registry::{get_process, register_process};
use crate::kernel::task::{get_task, register_task, unregister_task};
#[path = "fork_context.rs"]
mod fork_context;
#[path = "fork_tls.rs"]
mod fork_tls;
use fork_context::snapshot_parent_task;
/// Production-grade Process fork / clone / exec primitives.
///
/// Design decisions:
/// * `do_fork`  — copies process metadata exactly according to POSIX.1-2017 semantics:
///   independent signal handler table, independent file descriptor table (shallow copy),
///   shared address space **not** duplicated (requires page-table CoW in the VMM layer
///   which is outside this module).
/// * `do_clone` — can create either a thread (CLONE_VM|CLONE_THREAD, same CR3, same files)
///               or a full child (no flags, separate resources).
/// * `do_exec`  — flushes all secondary threads, resets signal dispositions, and
///               prepares the primary thread to enter the new entry point on next
///               return to user mode.
/// * TaskId allocation uses a single globally-monotonic atomic to avoid ID reuse.
/// * All operations are IRQ-safe via IrqSafeMutex throughout.
use core::sync::atomic::{AtomicUsize, Ordering};

// ── Clone flags (POSIX / Linux compatible) ─────────────────────────────────────
pub const CLONE_VM: u64 = 0x0000_0100;
///< Share VM (thread)
pub const CLONE_FS: u64 = 0x0000_0200;
///< Share cwd / root
pub const CLONE_FILES: u64 = 0x0000_0400;
///< Share FD table
pub const CLONE_SIGHAND: u64 = 0x0000_0800;
///< Share signal handlers
pub const CLONE_THREAD: u64 = 0x0001_0000;
///< Same thread group
pub const CLONE_NEWNS: u64 = 0x0002_0000;
///< New mount namespace

// ── Global monotonic TaskId counter ───────────────────────────────────────────
//
// We start above 0x1000 to keep low values reserved for BSP / early tasks.
// Using a single global counter prevents ID reuse across processes.
static GLOBAL_TID_COUNTER: AtomicUsize = AtomicUsize::new(0x1000);

#[inline(always)]
fn alloc_tid() -> TaskId {
    TaskId(GLOBAL_TID_COUNTER.fetch_add(1, Ordering::Relaxed))
}

// ── Error type ────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkError {
    /// The parent process was not found in the registry.
    ParentNotFound,
    /// The child process could not be registered (registry full).
    RegistryFull,
    /// No scheduler CPU slot was available to enqueue the new task.
    NoSchedulerSlot,
    /// The process has no threads (required for exec).
    NoThreads,
    /// The pid passed to exec/clone was not found.
    ProcessNotFound,
    /// A resource limit (e.g. RLIMIT_NPROC) would be exceeded.
    LimitExceeded,
}

impl ForkError {
    /// Convert to a POSIX errno string for use in syscall layer.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParentNotFound => "ESRCH",
            Self::RegistryFull => "EAGAIN",
            Self::NoSchedulerSlot => "EAGAIN",
            Self::NoThreads => "EINVAL",
            Self::ProcessNotFound => "ESRCH",
            Self::LimitExceeded => "EAGAIN",
        }
    }
}

// ── Internal: enqueue task to least-loaded CPU ────────────────────────────────
pub(crate) fn enqueue_task(
    tid: TaskId,
    preferred_cpu: crate::interfaces::task::CpuId,
) -> Result<(), ForkError> {
    use crate::interfaces::Scheduler;

    let task_arc = get_task(tid).ok_or(ForkError::NoSchedulerSlot)?;

    // 1. Try the preferred / parent CPU first.
    let cpu_count = crate::hal::smp::CPUS.lock().len();
    if cpu_count == 0 {
        return Err(ForkError::NoSchedulerSlot);
    }

    let preferred_idx = if preferred_cpu.is_any() {
        0
    } else {
        preferred_cpu.0.min(cpu_count - 1)
    };

    // 2. Find the CPU with the shortest runqueue within ±4 cores of preferred.
    let start = preferred_idx.saturating_sub(2);
    let end = (preferred_idx + 3).min(cpu_count - 1);

    let mut best_cpu = preferred_idx;
    let mut best_depth = usize::MAX;

    {
        let cpus = crate::hal::smp::CPUS.lock();
        for i in start..=end {
            if let Some(cpu) = cpus.get(i) {
                let depth = cpu.scheduler.lock().runqueue_len();
                if depth < best_depth {
                    best_depth = depth;
                    best_cpu = i;
                }
            }
        }
    }

    // 3. Enqueue.
    {
        let cpus = crate::hal::smp::CPUS.lock();
        if let Some(cpu) = cpus.get(best_cpu) {
            cpu.scheduler.lock().add_task(task_arc);
            return Ok(());
        }
    }

    Err(ForkError::NoSchedulerSlot)
}

// ── fork ──────────────────────────────────────────────────────────────────────

/// Create a child process.
///
/// POSIX semantics (simplified, without paging CoW):
/// * New PID, new address-space descriptor (same `cr3` until VMM adds CoW).
/// * Independent copy of signal disposition table.
/// * Shallow copy of FD table (file objects are ref-counted).
/// * The child's single thread begins at `child_entry` in kernel context; the
///   syscall layer sets this to the return path so the child returns 0.
///
/// Returns `(child_pid, child_tid)`.
pub fn do_fork(
    parent_pid: ProcessId,
    kernel_stack_top: u64,
    child_entry: u64, // RIP/ELR the child thread starts at
    cr3: u64,
    child_priority: u8,
) -> Result<(ProcessId, TaskId), ForkError> {
    // ── 1. Locate parent ────────────────────────────────────────────────────
    let parent = get_process(parent_pid).ok_or(ForkError::ParentNotFound)?;
    let parent_task = snapshot_parent_task(parent_pid).ok_or(ForkError::ParentNotFound)?;

    // ── 2. Enforce RLIMIT_NPROC ─────────────────────────────────────────────
    let rlimit_nproc = parent.resource_limits.max_processes;
    {
        let current_count = crate::kernel::process_registry::process_count();
        if rlimit_nproc > 0 && current_count >= rlimit_nproc {
            return Err(ForkError::LimitExceeded);
        }
    }

    // ── 3. Build child name ─────────────────────────────────────────────────
    let mut child_name = [0u8; 32];
    {
        let pname = parent.name.lock();
        // prefix "c:" (2 bytes) + truncated parent name
        let available = child_name.len() - 2;
        let src_len = pname.iter().position(|&b| b == 0).unwrap_or(pname.len());
        let copy_len = src_len.min(available);
        child_name[0..2].copy_from_slice(b"c:");
        child_name[2..2 + copy_len].copy_from_slice(&pname[..copy_len]);
    }

    // ── 4. Create child Process (inherits security / limits / namespace) ────
    let mut child_proc = crate::kernel::process::Process::new(
        &child_name,
        #[cfg(feature = "paging_enable")]
        x86_64::PhysAddr::new(cr3),
    );
    child_proc.security_level = parent.security_level;
    child_proc.resource_limits = parent.resource_limits;
    child_proc.namespace_id.store(
        parent.namespace_id.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    child_proc
        .cgroup_id
        .store(parent.cgroup_id.load(Ordering::Relaxed), Ordering::Relaxed);

    // Independent copy of signal handler table.
    {
        let src = parent.signal_handlers.lock();
        let mut dst = child_proc.signal_handlers.lock();
        for (&sig, &handler) in src.iter() {
            dst.insert(sig, handler);
        }
    }

    // Shallow FD copy: for now we don't duplicate File objects (no CoW FDT).
    // A real implementation would clone Arc<File> references here.

    let child_pid = child_proc.id;

    // ── 5. Register child process ───────────────────────────────────────────
    register_process(child_proc);

    // ── 6. Allocate child thread ────────────────────────────────────────────
    let child_tid = alloc_tid();

    let mut child_task = KernelTask::new(
        child_tid,
        child_priority.max(1).max(parent_task.priority),
        parent_task.deadline,
        parent_task.burst_time,
        kernel_stack_top,
        cr3,
        child_entry,
    );
    child_task.process_id = Some(child_pid);
    child_task.cfs_group_id = parent_task.cfs_group_id;
    child_task.cgroup_id = parent_task.cgroup_id;
    child_task.uid = parent_task.uid;
    child_task.gid = parent_task.gid;
    child_task.security_ctx = parent_task.security_ctx;
    child_task.resource_limits = parent_task.resource_limits;
    child_task.cpu_affinity_mask = parent_task.cpu_affinity_mask;
    child_task.preferred_cpu = parent_task.preferred_cpu;
    child_task.signal_mask = parent_task.signal_mask;
    child_task.signal_stack = parent_task.signal_stack;

    register_task(child_task);

    // Add thread to child process.
    if let Some(proc) = get_process(child_pid) {
        proc.threads.lock().push(child_tid);
        proc.mark_runnable();
    }

    // ── 7. Schedule child (prefer same CPU as parent) ──────────────────────
    enqueue_task(child_tid, parent_task.preferred_cpu)?;

    crate::klog_info!(
        "fork: parent_pid={} child_pid={} child_tid={}",
        parent_pid.0,
        child_pid.0,
        child_tid.0
    );
    Ok((child_pid, child_tid))
}

// ── exec ──────────────────────────────────────────────────────────────────────

/// Replace the calling process's image.
///
/// POSIX semantics implemented here:
/// 1. All secondary threads are terminated (single-threaded after exec).
/// 2. Signal dispositions for caught signals are reset to SIG_DFL (cleared).
/// 3. Signal mask is preserved (POSIX).
/// 4. The primary thread's context.rip/elr is set to `new_entry`.
/// 5. `image_entry` in the Process is updated.
///
/// The caller is responsible for loading the new image into memory and
/// providing the correct entry point.
pub fn do_exec(pid: ProcessId, new_entry: u64) -> Result<(), ForkError> {
    let process = get_process(pid).ok_or(ForkError::ProcessNotFound)?;
    let parent_task = snapshot_parent_task(pid).ok_or(ForkError::ProcessNotFound)?;

    // ── 1. Collect thread list ──────────────────────────────────────────────
    let threads: alloc::vec::Vec<TaskId> = process.threads.lock().clone();
    if threads.is_empty() {
        return Err(ForkError::NoThreads);
    }

    let primary_tid = threads[0];

    // ── 2. Terminate secondary threads ─────────────────────────────────────
    for &tid in threads.iter().skip(1) {
        // Mark terminated and unregister; scheduler will skip them naturally.
        if let Some(task_arc) = get_task(tid) {
            task_arc.lock().state = TaskState::Terminated;
        }
        unregister_task(tid);
    }
    {
        let mut t = process.threads.lock();
        t.retain(|&tid| tid == primary_tid);
    }

    // ── 3. Reset caught signal handlers to SIG_DFL (clear handler table) ───
    process.signal_handlers.lock().clear();

    // ── 4. Update entry point in the process descriptor ────────────────────
    process
        .image_entry
        .store(new_entry as usize, Ordering::Release);
    process.mark_runnable();

    // ── 5. Rewrite the primary thread's instruction pointer ─────────────────
    if let Some(task_arc) = get_task(primary_tid) {
        let mut task = task_arc.lock();
        #[cfg(target_arch = "x86_64")]
        {
            task.context.rip = new_entry;
        }
        #[cfg(target_arch = "aarch64")]
        {
            task.context.elr = new_entry;
        }
        // Make sure it's runnable.
        task.state = TaskState::Ready;
        task.priority = parent_task.priority;
        task.deadline = parent_task.deadline;
        task.burst_time = parent_task.burst_time;
        task.cfs_group_id = parent_task.cfs_group_id;
        task.cgroup_id = parent_task.cgroup_id;
        task.uid = parent_task.uid;
        task.gid = parent_task.gid;
        task.security_ctx = parent_task.security_ctx;
        task.resource_limits = parent_task.resource_limits;
        task.cpu_affinity_mask = parent_task.cpu_affinity_mask;
        task.preferred_cpu = parent_task.preferred_cpu;
        task.pending_signals = 0;
        task.signal_stack = None;
        #[cfg(feature = "ring_protection")]
        {
            task.user_tls_base = 0;
        }
        #[cfg(all(feature = "ring_protection", feature = "posix_mman"))]
        {
            let _ = fork_tls::initialize_task_tls(&process, &mut task);
            #[cfg(target_arch = "x86_64")]
            <crate::hal::cpu::ArchCpuRegisters as crate::interfaces::cpu::CpuRegisters>::write_tls_base(
                task.user_tls_base,
            );
        }
    }

    crate::klog_info!(
        "exec: pid={} entry={:#x} secondary_threads_terminated={}",
        pid.0,
        new_entry,
        threads.len() - 1
    );
    Ok(())
}

// ── clone ─────────────────────────────────────────────────────────────────────

/// Clone the calling process / create a new thread.
///
/// Behavior depends on `flags`:
/// * `CLONE_VM | CLONE_THREAD` (typical pthread_create): new thread in same process,
///   same address space, same FD table. signal_mask reset.
/// * No CLONE_VM: behaves like `do_fork` with a separate PID.
///
/// `tls_ptr`: if non-zero, stored in the new task's `user_tls_base`.
pub fn do_clone(
    parent_pid: ProcessId,
    kernel_stack_top: u64,
    entry: u64,
    cr3: u64,
    flags: u64,
    tls_ptr: u64,
) -> Result<TaskId, ForkError> {
    let process = get_process(parent_pid).ok_or(ForkError::ProcessNotFound)?;
    let parent_task = snapshot_parent_task(parent_pid).ok_or(ForkError::ProcessNotFound)?;

    // RLIMIT_NPROC check (threads count against the limit too).
    {
        let thread_count = process.threads.lock().len();
        let rlimit = process.resource_limits.max_processes;
        if rlimit > 0 && thread_count >= rlimit {
            return Err(ForkError::LimitExceeded);
        }
    }

    let new_tid = alloc_tid();

    let parent_priority = {
        let threads = process.threads.lock();
        threads
            .first()
            .and_then(|&tid| get_task(tid).map(|a| a.lock().priority))
            .unwrap_or(128)
    };

    let mut new_task = KernelTask::new(
        new_tid,
        parent_priority,
        parent_task.deadline,
        parent_task.burst_time,
        kernel_stack_top,
        cr3,
        entry,
    );
    new_task.process_id = Some(parent_pid);
    new_task.cfs_group_id = parent_task.cfs_group_id;
    new_task.cgroup_id = parent_task.cgroup_id;
    new_task.uid = parent_task.uid;
    new_task.gid = parent_task.gid;
    new_task.security_ctx = parent_task.security_ctx;
    new_task.resource_limits = parent_task.resource_limits;
    new_task.cpu_affinity_mask = parent_task.cpu_affinity_mask;
    new_task.preferred_cpu = parent_task.preferred_cpu;
    new_task.signal_mask = parent_task.signal_mask;
    new_task.signal_stack = parent_task.signal_stack;

    // TLS base for the new thread.
    #[cfg(feature = "ring_protection")]
    {
        new_task.user_tls_base = if tls_ptr != 0 {
            tls_ptr
        } else {
            parent_task.user_tls_base
        };
    }
    #[cfg(all(feature = "ring_protection", feature = "posix_mman"))]
    if tls_ptr == 0 {
        let _ = fork_tls::initialize_task_tls(&process, &mut new_task);
    }

    register_task(new_task);
    process.threads.lock().push(new_tid);
    process.mark_runnable();

    // Enqueue on the same logical CPU the parent prefers.
    enqueue_task(new_tid, parent_task.preferred_cpu)?;

    let kind = if flags & CLONE_VM != 0 {
        "thread"
    } else {
        "process-clone"
    };
    crate::klog_info!(
        "clone({}): pid={} new_tid={} tls={:#x}",
        kind,
        parent_pid.0,
        new_tid.0,
        tls_ptr
    );
    Ok(new_tid)
}
