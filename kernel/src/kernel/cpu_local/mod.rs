use crate::interfaces::task::{CpuId, TaskId};
use crate::modules::selector::ActiveScheduler;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Per-CPU Data Structure.
/// This struct is accessed via the GS_BASE (x86) or TPIDR_EL1 (ARM) register.
/// It MUST be pinned to a specific CPU core. Accessing it from another core is UB.

#[repr(C)] // Standard layout for assembly access
pub struct CpuLocal {
    pub cpu_id: CpuId, // Offset 0 (repr(transparent) => same as usize)
    #[cfg(feature = "ring_protection")]
    pub scratch: u64, // Offset 8 (Used for saving User RSP during syscall)
    #[cfg(feature = "ring_protection")]
    pub kernel_stack_top: core::sync::atomic::AtomicUsize, // Offset 16 (Top of Kernel Stack for this task)
    pub current_task: AtomicUsize, // Offset 8 or 24
    pub is_user_mode: core::sync::atomic::AtomicBool,
    pub heartbeat_tick: AtomicU64,
    pub idle_stack_pointer: AtomicUsize,
    // Each CPU holds its own scheduler instance + runqueue.
    pub scheduler: crate::kernel::sync::IrqSafeMutex<ActiveScheduler>,
    pub kernel_mode_depth: core::sync::atomic::AtomicU32,
}

impl CpuLocal {
    #[inline(always)]
    pub fn try_id() -> Option<usize> {
        // Safety: we only read the per-cpu base if the architecture register
        // has been initialized for the current CPU.
        unsafe { Self::try_get().map(|cpu| cpu.cpu_id.0) }
    }

    /// Try to access the current CpuLocal reference.
    /// Returns None if per-cpu base is not initialized yet.
    #[inline(always)]
    pub unsafe fn try_get() -> Option<&'static Self> {
        let base = crate::hal::HAL::read_per_cpu_base();
        if base == 0 {
            return None;
        }
        // Safety: the architecture-specific per-cpu base points to the
        // pinned CpuLocal allocation for the current CPU once initialized.
        Some(unsafe { &*(base as *const Self) })
    }

    /// Initialize the GS/TPIDR register to point to this struct.
    /// This function must be called once per core during boot.
    /// Safety: The reference must live for the lifetime of the OS (static).
    pub unsafe fn init(&'static self) {
        use crate::interfaces::HardwareAbstraction;
        let ptr = self as *const _ as usize;
        crate::hal::HAL::init_cpu_local(ptr);
    }

    /// Example: Get the current CPU ID very quickly
    #[inline(always)]
    pub fn id() -> usize {
        Self::try_id().unwrap_or(0)
    }

    #[inline(always)]
    pub fn id_typed() -> CpuId {
        CpuId(Self::id())
    }

    #[inline(always)]
    pub fn cpu_id_typed(&self) -> CpuId {
        self.cpu_id
    }

    #[inline(always)]
    pub fn current_task_id(&self) -> crate::interfaces::task::TaskId {
        crate::interfaces::task::TaskId(
            self.current_task
                .load(core::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Access the full struct reference
    /// Safety: Caller must ensure interrupts are disabled or preemption is off
    /// to avoid migrating to another CPU while holding this reference.
    #[inline(always)]
    pub unsafe fn get() -> &'static Self {
        unsafe { Self::try_get() }.expect("CpuLocal is not initialized")
    }

    /// Mark the start of kernel execution for the current task.
    /// Accounting is done if telemetry is enabled.
    /// Mark the entry into kernel mode from user mode or nested kernel mode.
    #[inline(always)]
    pub fn enter_kernel(&self) {
        #[cfg(feature = "telemetry")]
        {
            let now = crate::hal::HAL::get_time_ns();
            let tid = self.current_task_id();
            if tid.0 != 0 {
                if let Some(task_arc) = crate::kernel::task::get_task(tid) {
                    let mut task = task_arc.lock();
                    let delta = now.saturating_sub(task.last_mode_switch_ns);
                    
                    // If we were in user mode, attribute delta to user_ns.
                    // If we were already in kernel mode (depth > 0), attribute delta to system_ns.
                    if self.is_user_mode.load(Ordering::SeqCst) {
                        task.user_ns += delta;
                    } else {
                        task.system_ns += delta;
                    }
                    
                    task.time_consumed += delta;
                    task.last_mode_switch_ns = now;
                }
            }
        }

        self.kernel_mode_depth.fetch_add(1, Ordering::SeqCst);
        self.is_user_mode.store(false, Ordering::SeqCst);
    }

    /// Mark the exit from kernel mode. Only transitions to user mode if depth reaches zero.
    #[inline(always)]
    pub fn exit_kernel(&self) {
        #[cfg(feature = "telemetry")]
        {
            let now = crate::hal::HAL::get_time_ns();
            let tid = self.current_task_id();
            if tid.0 != 0 {
                if let Some(task_arc) = crate::kernel::task::get_task(tid) {
                    let mut task = task_arc.lock();
                    let delta = now.saturating_sub(task.last_mode_switch_ns);
                    
                    // We are in kernel mode, so delta always goes to system_ns.
                    task.system_ns += delta;
                    task.time_consumed += delta;
                    task.last_mode_switch_ns = now;
                }
            }
        }

        let old_depth = self.kernel_mode_depth.fetch_sub(1, Ordering::SeqCst);
        if old_depth == 1 {
            // We are returning to the top-level (user space).
            self.is_user_mode.store(true, Ordering::SeqCst);
        }
    }

    /// Update telemetry during a context switch.
    /// This ensures that the time spent by the old task in the kernel is credited
    /// and the new task's baseline is established.
    #[inline(always)]
    pub fn on_context_switch(&self, old_tid: TaskId, new_tid: TaskId) {
        #[cfg(feature = "telemetry")]
        {
            let now = crate::hal::HAL::get_time_ns();
            if let Some(task_arc) = crate::kernel::task::get_task(old_tid) {
                let mut task = task_arc.lock();
                let delta = now.saturating_sub(task.last_mode_switch_ns);
                // Context switches always happen in kernel mode.
                task.system_ns += delta;
                task.time_consumed += delta;
                task.last_mode_switch_ns = now;
            }
            if let Some(task_arc) = crate::kernel::task::get_task(new_tid) {
                let mut task = task_arc.lock();
                task.last_mode_switch_ns = now;
            }
        }
    }
}
