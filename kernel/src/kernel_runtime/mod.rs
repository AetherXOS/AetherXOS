//! Kernel runtime orchestration module.
//!
//! Coordinates the lifecycle of the kernel: boot, heap setup, scheduler initialization,
//! VFS mounting, driver loading, and syscall integration.
//!
//! # Architecture
//!
//! This module serves as the central wiring layer that connects all kernel subsystems.
//! It is structured into logical sub-modules:
//!
//! - `boot_sequence/`: Early boot stages and initialization ordering
//! - `boot_flow/`: Boot-stage device, IRQ, memory routing
//! - `boot_info/`: Bootloader-provided info (memory map, framebuffer, cmdline, firmware)
//! - `platform/`: Platform-level initialization (CPU, memory map, ...)
//! - `platform_support/`: Architecture/platform specific support code
//! - `drivers_init/`: Driver subsystem initialization
//! - `heap.rs`: Heap allocator setup
//! - `memory_integration.rs`: Memory subsystem integration hooks
//! - `scheduler_integration.rs`: Scheduler extension integration
//! - `vfs_integration.rs`: VFS subsystem integration
//! - `syscall_integration.rs`: Syscall path integration
//! - `service_integration.rs`: Service subsystem integration
//! - `integration_utils.rs`: Shared utilities for integration points

pub(crate) mod heap;
pub(crate) mod memory_integration;
pub(crate) mod scheduler_integration;
pub(crate) mod service_integration;
pub(crate) mod syscall_integration;
pub(crate) mod vfs_integration;
pub(crate) mod integration_utils;

pub mod boot_sequence;
pub mod boot_flow;
pub mod boot_info;
pub mod platform;
pub mod drivers_init;
pub mod platform_support;
pub(crate) mod interrupts;

use core::sync::atomic::{AtomicBool, Ordering};

static HEAP_READY: AtomicBool = AtomicBool::new(false);
static RUNTIME_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns `true` once the heap allocator is ready for use.
#[inline(always)]
pub fn heap_ready() -> bool {
    HEAP_READY.load(Ordering::Acquire)
}

/// Marks the heap as fully initialized.
pub(crate) fn set_heap_ready() {
    HEAP_READY.store(true, Ordering::Release);
}

/// Returns `true` once the full runtime has been initialized.
#[inline(always)]
pub fn is_runtime_initialized() -> bool {
    RUNTIME_INITIALIZED.load(Ordering::Acquire)
}

/// Marks the kernel runtime as fully initialized.
pub fn mark_runtime_initialized() {
    RUNTIME_INITIALIZED.store(true, Ordering::Release);
}

/// Main kernel runtime entry point.
/// Called from `kernel::startup` after HAL early init.
pub struct KernelRuntime;

impl KernelRuntime {
    /// Create a new kernel runtime instance.
    pub fn new() -> Self {
        Self
    }

    /// Run the full kernel initialization sequence.
    /// This is the top-level orchestration entry point.
    pub fn run(self) -> ! {
        // Delegate to the boot_sequence orchestrator which manages
        // the full initialization pipeline: HAL early init, heap setup,
        // scheduler init, VFS mounting, and driver loading.
        crate::kernel_runtime::boot_sequence::run_boot_sequence()
    }
}
