use crate::interfaces::security::{ResourceLimits, SecurityContext};
use alloc::sync::Arc;
use crate::kernel::sync::IrqSafeMutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub usize);

impl core::fmt::Display for TaskId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(pub usize);

impl core::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuId(pub usize);

impl core::fmt::Display for CpuId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl CpuId {
    pub const ANY: Self = Self(usize::MAX);

    #[inline(always)]
    pub const fn is_any(self) -> bool {
        self.0 == usize::MAX
    }
}

/// Simplified Task Structure
#[derive(Debug, Clone)]
pub struct KernelTask {
    pub id: TaskId,
    pub name: alloc::string::String,
    pub priority: u8,
    pub deadline: u64,   // For RealTime/EDF
    pub burst_time: u64, // For SJF/Batch
    pub rt_group_id: u16,
    pub rt_budget_ns: u64,
    pub rt_period_ns: u64,
    /// CFS group ID for hierarchical group scheduling (0 = root group).
    pub cfs_group_id: u16,
    /// Cgroup ID for resource accounting via CgroupManager (1 = root cgroup).
    pub cgroup_id: u64,

    /// Optional ownership link to a process.
    pub process_id: Option<ProcessId>,

    pub uid: u32,
    pub gid: u32,

    /// Per-task security context (identity, capabilities, MAC level, namespace).
    pub security_ctx: SecurityContext,
    /// Resource limits enforced for this task's owning process.
    pub resource_limits: ResourceLimits,

    // --- Advanced Scheduler Metrics (Telemetry) ---
    #[cfg(feature = "telemetry")]
    pub time_consumed: u64, // Total CPU time used (ns)
    #[cfg(feature = "telemetry")]
    pub time_slice_left: u64, // Remaining time in current slice (ns)
    #[cfg(feature = "telemetry")]
    pub last_ran: u64, // Timestamp when it last ran (for starvation avoidance)

    pub state: TaskState,

    // --- CPU Affinity Metadata ---
    // Bitmask of CPUs this task is allowed to run on.
    // Bit i => CPU i allowed.
    pub cpu_affinity_mask: u64,
    // Soft preference for scheduler/load-balancer placement.
    pub preferred_cpu: CpuId,

    // --- OS Context (Arch-Specific) ---
    // Stack and CR3 needed even in Ring 0 mode, but we can simplify if needed.
    // For now we keep them always: stack needed for context switch, page_table needed for memory space isolation.
    // If we wanted to go full unikernel (Single Address Space), we could remove `page_table_root`.
    #[cfg(feature = "ring_protection")]
    pub user_stack_pointer: u64, // Only needed if we switch to Ring 3
    #[cfg(feature = "ring_protection")]
    pub user_tls_base: u64,
    pub kernel_stack_pointer: u64, // Always needed for Ring 0 stack
    pub page_table_root: u64,      // CR3 (Physical Address)
    pub context: Context,

    pub signal_stack: Option<SignalStack>,
    pub pending_signals: u64,
    pub signal_mask: u64,
    pub clear_child_tid: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SignalStack {
    pub ss_sp: u64,
    pub ss_flags: i32,
    pub ss_size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelTaskBootstrapSpec {
    pub id: TaskId,
    pub priority: u8,
    pub deadline: u64,
    pub burst_time: u64,
    pub kernel_stack: u64,
    pub cr3: u64,
    pub entry: u64,
}

impl KernelTask {
    extern "C" fn initial_task_return_trap() -> ! {
        panic!("kernel task entry returned unexpectedly")
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn initial_stack_frame_image(entry: u64) -> [u64; 8] {
        [
            0u64,
            0u64,
            0u64,
            0u64,
            0u64,
            0u64,
            entry,
            Self::initial_task_return_trap as *const () as usize as u64,
        ]
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(never)]
    unsafe fn prepare_initial_kernel_stack_frame(kernel_stack: u64, entry: u64) -> u64 {
        crate::kernel::debug_trace::record_with_metadata(
            "task.stack",
            "begin",
            Some(kernel_stack),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(target_os = "none")]
        crate::hal::x86_64::serial::write_trace("task.stack", "begin");
        const SAVED_QWORDS: usize = 8;
        let frame_size = SAVED_QWORDS * core::mem::size_of::<u64>();
        crate::kernel::debug_trace::record_with_metadata(
            "task.stack",
            "frame_size_ready",
            Some(frame_size as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(target_os = "none")]
        crate::hal::x86_64::serial::write_trace("task.stack", "frame_size_ready");
        let frame_top = match (kernel_stack as usize).checked_sub(frame_size) {
            Some(v) => v,
            None => {
                crate::kernel::debug_trace::record_fault("task.stack", "underflow", None);
                #[cfg(target_os = "none")]
                crate::hal::x86_64::serial::write_trace("task.stack", "underflow");
                return 0;
            }
        };
        crate::kernel::debug_trace::record_with_metadata(
            "task.stack",
            "frame_top_ready",
            Some(frame_top as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(target_os = "none")]
        crate::hal::x86_64::serial::write_trace("task.stack", "frame_top_ready");
        let frame = frame_top as *mut u64;
        let frame_image = Self::initial_stack_frame_image(entry);
        // Restored by x86_64::context_switch:
        // r15, r14, r13, r12, rbx, rbp, ret(entry), synthetic return address
        unsafe {
            crate::kernel::debug_trace::record_with_metadata(
                "task.stack",
                "frame_image_ready",
                Some(frame_image[6]),
                false,
                crate::kernel::debug_trace::TraceSeverity::Trace,
                crate::kernel::debug_trace::TraceCategory::Task,
            );
            #[cfg(target_os = "none")]
            crate::hal::x86_64::serial::write_trace("task.stack", "frame_image_ready");
            crate::kernel::debug_trace::record_with_metadata(
                "task.stack",
                "return_trap_ready",
                Some(frame_image[7]),
                false,
                crate::kernel::debug_trace::TraceSeverity::Trace,
                crate::kernel::debug_trace::TraceCategory::Task,
            );
            #[cfg(target_os = "none")]
            crate::hal::x86_64::serial::write_trace("task.stack", "return_trap_ready");
            crate::kernel::debug_trace::record_with_metadata(
                "task.stack",
                "bulk_copy_begin",
                None,
                false,
                crate::kernel::debug_trace::TraceSeverity::Trace,
                crate::kernel::debug_trace::TraceCategory::Task,
            );
            #[cfg(target_os = "none")]
            crate::hal::x86_64::serial::write_trace("task.stack", "bulk_copy_begin");
            core::ptr::copy_nonoverlapping(frame_image.as_ptr(), frame, frame_image.len());
            crate::kernel::debug_trace::record_with_metadata(
                "task.stack",
                "bulk_copy_returned",
                Some(frame_top as u64),
                false,
                crate::kernel::debug_trace::TraceSeverity::Trace,
                crate::kernel::debug_trace::TraceCategory::Task,
            );
            #[cfg(target_os = "none")]
            crate::hal::x86_64::serial::write_trace("task.stack", "bulk_copy_returned");
        }
        crate::kernel::debug_trace::record_with_metadata(
            "task.stack",
            "returned",
            Some(frame_top as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        frame_top as u64
    }

    #[inline(always)]
    pub const fn bootstrap_spec(
        id: TaskId,
        priority: u8,
        deadline: u64,
        burst_time: u64,
        kernel_stack: u64,
        cr3: u64,
        entry: u64,
    ) -> KernelTaskBootstrapSpec {
        KernelTaskBootstrapSpec {
            id,
            priority,
            deadline,
            burst_time,
            kernel_stack,
            cr3,
            entry,
        }
    }

    #[inline(never)]
    pub fn new_from_spec(spec: KernelTaskBootstrapSpec) -> Self {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] task.new raw begin\n");
        crate::kernel::debug_trace::record_with_metadata(
            "task.new",
            "begin",
            Some(spec.id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_trace("task.new", "begin");
        let mut ctx = Context::default();

        #[cfg(target_arch = "x86_64")]
        {
            use crate::kernel::syscalls::syscalls_consts::x86;
            ctx.rip = spec.entry;
            crate::kernel::debug_trace::record_with_metadata(
                "task.new",
                "stack_prep_begin",
                Some(spec.entry),
                false,
                crate::kernel::debug_trace::TraceSeverity::Trace,
                crate::kernel::debug_trace::TraceCategory::Task,
            );
            #[cfg(target_os = "none")]
            crate::hal::x86_64::serial::write_trace("task.new", "stack_prep_begin");
            let prepared_stack = if spec.kernel_stack != 0 {
                unsafe { Self::prepare_initial_kernel_stack_frame(spec.kernel_stack, spec.entry) }
            } else {
                0
            };
            crate::kernel::debug_trace::record_with_metadata(
                "task.new",
                "stack_prep_returned",
                Some(prepared_stack),
                false,
                crate::kernel::debug_trace::TraceSeverity::Trace,
                crate::kernel::debug_trace::TraceCategory::Task,
            );
            #[cfg(target_os = "none")]
            crate::hal::x86_64::serial::write_trace("task.new", "stack_prep_returned");
            ctx.rsp = prepared_stack;
            // RFLAGS: IF enabled (allow interrupts) and reserved bit set
            ctx.rflags = x86::RFLAGS_IF_RESERVED;
        }

        #[cfg(target_arch = "aarch64")]
        {
            ctx.elr = spec.entry;
            ctx.sp = spec.kernel_stack;
            // SPSR: EL1h with IRQ unmasked
            ctx.spsr = 0x3c5;
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            ctx.pc = spec.entry;
            ctx.sp = spec.kernel_stack;
        }

        let task = Self {
            id: spec.id,
            name: alloc::format!("task-{}", spec.id.0),
            priority: spec.priority,
            deadline: spec.deadline,
            burst_time: spec.burst_time,
            rt_group_id: 0,
            rt_budget_ns: 0,
            rt_period_ns: 0,
            cfs_group_id: 0,
            cgroup_id: 1, // root cgroup (ROOT_CGROUP_ID = 1)
            process_id: None,
            uid: 0,
            gid: 0,
            security_ctx: SecurityContext::kernel(),
            resource_limits: ResourceLimits::unlimited(),

            #[cfg(feature = "telemetry")]
            time_consumed: 0,
            #[cfg(feature = "telemetry")]
            time_slice_left: 0,
            #[cfg(feature = "telemetry")]
            last_ran: 0,

            state: TaskState::Ready,
            cpu_affinity_mask: u64::MAX,
            preferred_cpu: CpuId::ANY,

            #[cfg(feature = "ring_protection")]
            user_stack_pointer: 0,
            #[cfg(feature = "ring_protection")]
            user_tls_base: 0,

            kernel_stack_pointer: {
                #[cfg(target_arch = "x86_64")]
                {
                    ctx.rsp
                }
                #[cfg(not(target_arch = "x86_64"))]
                {
                    spec.kernel_stack
                }
            },
            page_table_root: spec.cr3,
            context: ctx,
            signal_stack: None,
            pending_signals: 0,
            signal_mask: 0,
            clear_child_tid: 0,
        };
        crate::kernel::debug_trace::record_with_metadata(
            "task.new",
            "returned",
            Some(task.kernel_stack_pointer),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_trace("task.new", "returned");
        task
    }

    pub fn new(
        id: TaskId,
        priority: u8,
        deadline: u64,
        burst_time: u64,
        kernel_stack: u64,
        cr3: u64,
        entry: u64,
    ) -> Self {
        Self::new_from_spec(Self::bootstrap_spec(
            id,
            priority,
            deadline,
            burst_time,
            kernel_stack,
            cr3,
            entry,
        ))
    }

    #[inline(never)]
    pub fn new_shared_from_spec(spec: KernelTaskBootstrapSpec) -> Arc<IrqSafeMutex<Self>> {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] task.shared raw begin\n");
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "begin",
            Some(spec.id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_trace("task.shared", "begin");
        let task = Self::new_from_spec(spec);
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "task_ready",
            None,
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_trace("task.shared", "task_ready");
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "mutex_wrap_begin",
            None,
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        let mutex = IrqSafeMutex::new(task);
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "mutex_wrap_returned",
            None,
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "arc_alloc_begin",
            None,
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        let shared = Arc::new(mutex);
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "arc_alloc_returned",
            None,
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "returned",
            None,
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] task.shared returned\n");
        shared
    }

    pub fn new_shared(
        id: TaskId,
        priority: u8,
        deadline: u64,
        burst_time: u64,
        kernel_stack: u64,
        cr3: u64,
        entry: u64,
    ) -> Arc<IrqSafeMutex<Self>> {
        Self::new_shared_from_spec(Self::bootstrap_spec(
            id,
            priority,
            deadline,
            burst_time,
            kernel_stack,
            cr3,
            entry,
        ))
    }

    #[inline(never)]
    pub fn new_shared_bootstrap(
        id: TaskId,
        priority: u8,
        deadline: u64,
        burst_time: u64,
        kernel_stack: u64,
        cr3: u64,
        entry: u64,
    ) -> Arc<IrqSafeMutex<Self>> {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] task.shared direct begin\n");
        let spec = Self::bootstrap_spec(
            id,
            priority,
            deadline,
            burst_time,
            kernel_stack,
            cr3,
            entry,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] task.shared direct spec returned\n");
        Self::new_shared_from_spec(spec)
    }

    #[inline(always)]
    pub fn can_run_on_cpu(&self, cpu_id: usize) -> bool {
        if cpu_id >= 64 {
            return false;
        }
        let bit = 1u64 << cpu_id;
        (self.cpu_affinity_mask & bit) != 0
    }

    #[inline(always)]
    pub fn can_run_on_cpu_id(&self, cpu_id: CpuId) -> bool {
        self.can_run_on_cpu(cpu_id.0)
    }

    #[inline(always)]
    pub fn with_affinity_mask(mut self, mask: u64) -> Self {
        self.cpu_affinity_mask = if mask == 0 { u64::MAX } else { mask };
        self
    }

    #[inline(always)]
    pub fn with_preferred_cpu(mut self, cpu_id: usize) -> Self {
        self.preferred_cpu = CpuId(cpu_id);
        self
    }

    #[inline(always)]
    pub fn with_preferred_cpu_id(self, cpu_id: CpuId) -> Self {
        let mut out = self;
        out.preferred_cpu = cpu_id;
        out
    }

    #[inline(always)]
    pub fn with_rt_group(mut self, group_id: u16) -> Self {
        self.rt_group_id = group_id;
        self
    }

    #[inline(always)]
    pub fn with_rt_budget(mut self, budget_ns: u64, period_ns: u64) -> Self {
        self.rt_budget_ns = budget_ns;
        self.rt_period_ns = period_ns;
        self
    }
}

/// Architecture-specific CPU Context (Registers)
#[derive(Debug, Clone, Default, Copy)]
#[repr(C)]
#[cfg(target_arch = "x86_64")]
pub struct Context {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[cfg(target_arch = "x86_64")]
impl Context {
    /// Return the instruction pointer (arch-agnostic accessor).
    #[inline(always)]
    pub fn rip(&self) -> u64 {
        self.rip
    }
}

/// Architecture-specific CPU Context (Registers) — AArch64
#[derive(Debug, Clone, Default, Copy)]
#[repr(C)]
#[cfg(target_arch = "aarch64")]
pub struct Context {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64, // FP
    pub x30: u64, // LR (return address)
    pub sp: u64,
    pub elr: u64,  // Exception Link Register (PC to return to)
    pub spsr: u64, // Saved Program Status Register
}

#[cfg(target_arch = "aarch64")]
impl Context {
    #[inline(always)]
    pub fn rip(&self) -> u64 {
        self.elr
    }
}

/// Fallback for unsupported architectures
#[derive(Debug, Clone, Default, Copy)]
#[repr(C)]
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub struct Context {
    pub pc: u64,
    pub sp: u64,
    pub flags: u64,
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
impl Context {
    #[inline(always)]
    pub fn rip(&self) -> u64 {
        self.pc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

#[cfg(test)]
mod tests;
