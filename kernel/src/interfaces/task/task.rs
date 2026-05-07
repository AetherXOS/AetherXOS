use super::context::*;
use super::ids::*;
use super::state::*;
use crate::interfaces::security::{ResourceLimits, SecurityContext};
use crate::kernel::sync::IrqSafeMutex;
use alloc::sync::Arc;

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

    pub expected_start_ns: u64,
    pub actual_start_ns: u64,

    /// Optional ownership link to a process.
    pub process_id: Option<ProcessId>,
    pub process_group_id: TaskId,
    pub session_id: TaskId,

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
    pub user_ns: u64, // User mode time (ns)
    #[cfg(feature = "telemetry")]
    pub system_ns: u64, // System/Kernel mode time (ns)
    #[cfg(feature = "telemetry")]
    pub last_mode_switch_ns: u64, // Timestamp of last mode transition
    #[cfg(feature = "telemetry")]
    pub time_slice_left: u64, // Remaining time in current slice (ns)
    #[cfg(feature = "telemetry")]
    pub last_ran: u64, // Timestamp when it last ran (for starvation avoidance)

    pub state: TaskState,

    // --- CPU Affinity Metadata ---
    pub cpu_affinity_mask: u64,
    pub preferred_cpu: CpuId,

    // --- OS Context (Arch-Specific) ---
    pub kernel_stack_pointer: u64,
    pub page_table_root: u64,
    pub context: Context,

    pub signal_queue: Arc<IrqSafeMutex<crate::kernel::signal::queue::SignalQueue>>,
    pub signal_stack: Option<SignalStack>,
    pub signal_mask: u64,
    pub clear_child_tid: usize,

    #[cfg(feature = "ring_protection")]
    pub user_stack_pointer: u64,
    #[cfg(feature = "ring_protection")]
    pub user_tls_base: u64,

    pub pending_signals: u64,
    pub handling_signals: u64,
    pub signal_stack_active: bool,
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
    /// Prepare the initial stack frame for task execution
    pub unsafe fn prepare_initial_kernel_stack_frame(kernel_stack: u64, entry: u64) -> u64 {
        let sp = kernel_stack & !0xF;
        let sp = sp - 8;
        unsafe {
            *(sp as *mut u64) = entry;
        }
        sp
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
        crate::hal::serial::write_raw("[EARLY SERIAL] task.new raw begin\n");
        crate::kernel::debug_trace::record_with_metadata(
            "task.new",
            "begin",
            Some(spec.id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::serial::write_trace("task.new", "begin");
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
            crate::hal::serial::write_trace("task.new", "stack_prep_begin");
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
            crate::hal::serial::write_trace("task.new", "stack_prep_returned");
            ctx.rsp = prepared_stack;
            ctx.rflags = x86::RFLAGS_IF_RESERVED;
        }

        #[cfg(target_arch = "aarch64")]
        {
            ctx.elr = spec.entry;
            ctx.sp = spec.kernel_stack;
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
            cgroup_id: 1,

            expected_start_ns: 0,
            actual_start_ns: 0,

            process_id: None,
            process_group_id: spec.id,
            session_id: spec.id,
            uid: 0,
            gid: 0,
            security_ctx: SecurityContext::kernel(),
            resource_limits: ResourceLimits::unlimited(),

            #[cfg(feature = "telemetry")]
            time_consumed: 0,
            #[cfg(feature = "telemetry")]
            user_ns: 0,
            #[cfg(feature = "telemetry")]
            system_ns: 0,
            #[cfg(feature = "telemetry")]
            last_mode_switch_ns: crate::hal::HAL::get_time_ns(),
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
            handling_signals: 0,
            signal_stack_active: false,
            clear_child_tid: 0,
            signal_queue: Arc::new(IrqSafeMutex::new(
                crate::kernel::signal::queue::SignalQueue::new(),
            )),
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
        crate::hal::serial::write_trace("task.new", "returned");
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
        crate::hal::serial::write_raw("[EARLY SERIAL] task.shared raw begin\n");
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "begin",
            Some(spec.id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::serial::write_trace("task.shared", "begin");
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
        crate::hal::serial::write_trace("task.shared", "task_ready");
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
        let shared_clone = shared.clone();
        crate::kernel::debug_trace::record_with_metadata(
            "task.shared",
            "returned",
            None,
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Task,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::serial::write_raw("[EARLY SERIAL] task.shared returned\n");
        shared_clone
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
        crate::hal::serial::write_raw("[EARLY SERIAL] task.shared direct begin\n");
        let spec =
            Self::bootstrap_spec(id, priority, deadline, burst_time, kernel_stack, cr3, entry);
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::serial::write_raw("[EARLY SERIAL] task.shared direct spec returned\n");
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
