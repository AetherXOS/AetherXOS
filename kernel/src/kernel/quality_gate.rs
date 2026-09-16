//! Kernel quality gate and invariant verification.
//!
//! Provides runtime assertions and compile-time checks that enforce
//! architectural invariants and code quality constraints.

use core::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// Invariant Tracking
// ---------------------------------------------------------------------------

/// Tracks the initialization state of a subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitState {
    Uninitialized,
    Initializing,
    Ready,
    Failed,
}

/// A lock-free initialization flag that can only be set once.
pub struct OnceInit {
    state: AtomicBool,
}

impl OnceInit {
    pub const fn new() -> Self {
        Self { state: AtomicBool::new(false) }
    }

    pub fn try_init(&self) -> bool {
        self.state.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).is_ok()
    }

    pub fn is_initialized(&self) -> bool {
        self.state.load(Ordering::Acquire)
    }

    /// # Safety
    /// Must only be called when no other thread can access this flag.
    pub unsafe fn reset(&self) {
        self.state.store(false, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Assertion Macros
// ---------------------------------------------------------------------------

/// Assert a precondition is met at runtime (debug builds only).
#[macro_export]
macro_rules! precondition {
    ($cond:expr) => {
        debug_assert!($cond, "[PRECONDITION] {}", stringify!($cond));
    };
}

/// Assert a postcondition is met at runtime (debug builds only).
#[macro_export]
macro_rules! postcondition {
    ($cond:expr) => {
        debug_assert!($cond, "[POSTCONDITION] {}", stringify!($cond));
    };
}

/// Assert an invariant is met at runtime (debug builds only).
#[macro_export]
macro_rules! invariant {
    ($cond:expr) => {
        debug_assert!($cond, "[INVARIANT] {}", stringify!($cond));
    };
}

// ---------------------------------------------------------------------------
// Global Invariant Checks
// ---------------------------------------------------------------------------

pub struct KernelInvariants;

impl KernelInvariants {
    pub fn check_scheduler_invariants() {
        #[cfg(debug_assertions)]
        {
            let cpu = crate::kernel::cpu_local::CpuLocal::id();
            let tid = crate::kernel::task::current_task_id();
            invariant!(tid.0 != 0 || cpu == 0);
        }
    }

    pub fn check_memory_invariants() {
        #[cfg(debug_assertions)]
        {
            invariant!(crate::kernel_runtime::heap_ready());
        }
    }
}

// ---------------------------------------------------------------------------
// Safety Audit Marker
// ---------------------------------------------------------------------------

/// Marker trait for types that have undergone a safety audit.
///
/// # Safety
/// Implementing this trait asserts a safety audit has been completed.
pub unsafe trait SafetyAudited {
    const AUDIT_REVISION: &'static str;
}
