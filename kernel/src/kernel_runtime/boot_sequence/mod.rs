//! boot_sequence module.

mod diagnostics;
mod prelude;
mod self_test;

pub(super) use diagnostics::log_boot_diagnostics;
pub(crate) use prelude::BootPrelude;
pub(super) use prelude::{
    finalize_boot_prelude, initialize_boot_prelude, log_linked_probe_boot,
    write_stage_serial_marker,
};
pub(super) use self_test::assert_boot_self_tests;

/// Entry point for the kernel boot sequence.
///
/// Orchestrates the full initialization pipeline:
/// 1. Boot prelude (memory map, framebuffer, serial, etc.)
/// 2. HAL early initialization
/// 3. Heap bootstrap
/// 4. Scheduler and task system initialization
/// 5. Interrupt handlers registration
/// 6. Init service spawning
/// 7. Main idle loop
///
/// # Safety
///
/// This function must only be called once during kernel startup, from a
/// single-CPU context before SMP is initialized.
pub fn run_boot_sequence() -> ! {
    crate::kernel::startup::mark_stage(crate::kernel::startup::StartupStage::BootStart);

    // Stage 1: Boot prelude + heap init
    initialize_boot_prelude();
    {
        use crate::modules::allocators::selector::ActiveHeapAllocator;
        let allocator = ActiveHeapAllocator::new();
        super::heap::init_heap(&allocator);
        super::heap::finalize_heap_bootstrap();
    }
    crate::kernel::startup::mark_stage(crate::kernel::startup::StartupStage::HeapInit);

    // Stage 2: HAL early init
    crate::hal::HAL::early_init();
    crate::kernel::startup::mark_stage(crate::kernel::startup::StartupStage::HalEarlyInit);

    // Stage 3: Self-tests
    assert_boot_self_tests();

    // Stage 4: Init services + scheduler
    crate::kernel::startup::spawn_init_services();
    #[cfg(feature = "schedulers")]
    {
        let sched = crate::modules::selector::bootstrap_active_scheduler();
        *crate::modules::selector::GLOBAL_SCHEDULER.lock() = Some(sched);
    }
    crate::kernel::startup::mark_stage(crate::kernel::startup::StartupStage::MainLoopEntered);

    // Stage 5: Enable interrupts (if bare-metal)
    #[cfg(target_os = "none")]
    crate::hal::HAL::enable_interrupts();

    // Log completion
    #[cfg(feature = "debug_observability_boot")]
    {
        crate::klog_info!("Kernel boot sequence complete");
        log_boot_diagnostics();
    }
    super::mark_runtime_initialized();

    // Idle loop: scheduler picks tasks via timer interrupts
    loop {
        crate::kernel::idle_once();
    }
}
