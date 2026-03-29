use crate::interfaces::task::CpuId;
use crate::modules::selector::ActiveScheduler;
use core::sync::atomic::{AtomicU64, AtomicUsize};

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
    pub heartbeat_tick: AtomicU64,
    pub idle_stack_pointer: AtomicUsize,
    // Each CPU holds its own scheduler instance + runqueue.
    pub scheduler: crate::kernel::sync::IrqSafeMutex<ActiveScheduler>,
}

// Global array of pointers to CpuLocal structs (for cross-cpu wakeups if needed)
// static CPU_LOCALS: [Option<&CpuLocal>; MAX_CPUS] = [None; MAX_CPUS];

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
        #[cfg(target_arch = "x86_64")]
        {
            use crate::interfaces::cpu::CpuRegisters;
            let base = crate::hal::cpu::ArchCpuRegisters::read_per_cpu_base();
            if base == 0 {
                return None;
            }
            // Safety: the architecture-specific per-cpu base points to the
            // pinned CpuLocal allocation for the current CPU once initialized.
            return Some(unsafe { &*(base as *const Self) });
        }

        #[cfg(target_arch = "aarch64")]
        {
            let ptr: u64;
            // Safety: reading TPIDR_EL1 is valid in kernel context and does not
            // modify machine state.
            unsafe {
                core::arch::asm!("mrs {}, tpidr_el1", out(reg) ptr);
            }
            if ptr == 0 {
                return None;
            }
            // Safety: TPIDR_EL1 is initialized to the current CPU's CpuLocal.
            return Some(unsafe { &*(ptr as *const Self) });
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            None
        }
    }

    /// Initialize the GS/TPIDR register to point to this struct.
    /// This function must be called once per core during boot.
    /// Safety: The reference must live for the lifetime of the OS (static).
    pub unsafe fn init(&'static self) {
        #[cfg(target_arch = "x86_64")]
        {
            use x86_64::registers::model_specific::GsBase;
            use x86_64::VirtAddr;
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cpu local gsbase write begin\n");
            let ptr = self as *const _ as u64;
            GsBase::write(VirtAddr::new(ptr));
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cpu local gsbase write returned\n");
        }

        #[cfg(target_arch = "aarch64")]
        {
            // Write to TPIDR_EL1
            let ptr = self as *const _ as u64;
            unsafe {
                core::arch::asm!("msr tpidr_el1, {}", in(reg) ptr);
            }
        }
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
}
