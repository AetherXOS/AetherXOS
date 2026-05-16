extern crate alloc;

// ── Architecture imports ──────────────────────────────────────────────────────
use crate::core::log;
use aethercore::hal::Hal;

// ── Phase 6: Boot Infrastructure Integration ──────────────────────────────────
use crate::interfaces::boot::BootStage;

// ── IRQ dispatcher (x86_64 only) ─────────────────────────────────────────────
#[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
use aethercore::modules::dispatcher::selector::ActiveDispatcher;

// ── Sub-modules ───────────────────────────────────────────────────────────────
#[path = "kernel_runtime/boot_flow/mod.rs"]
mod boot_flow;
#[path = "kernel_runtime/boot_info/mod.rs"]
mod boot_info;
#[path = "kernel_runtime/boot_integration.rs"]
pub mod boot_integration;
#[path = "kernel_runtime/boot_sequence/mod.rs"]
mod boot_sequence;
#[path = "kernel_runtime/drivers_init/mod.rs"]
mod drivers_init;
#[path = "kernel_runtime/heap.rs"]
mod heap;
#[path = "kernel_runtime/integration_utils.rs"]
pub mod integration_utils;
#[path = "kernel_runtime/interrupts/mod.rs"]
mod interrupts;
#[path = "kernel_runtime/main_loop/mod.rs"]
mod main_loop;
#[path = "kernel_runtime/memory_integration.rs"]
pub mod memory_integration;
#[path = "kernel_runtime/networking/mod.rs"]
mod networking;
#[path = "kernel_runtime/platform.rs"]
mod platform;
#[path = "kernel_runtime/platform_support/mod.rs"]
mod platform_support;
#[path = "kernel_runtime/runtime_boot.rs"]
mod runtime_boot;
#[path = "kernel_runtime/scheduler_integration.rs"]
pub mod scheduler_integration;
#[path = "kernel_runtime/service_integration.rs"]
pub mod service_integration;
#[path = "kernel_runtime/syscall_integration.rs"]
pub mod syscall_integration;
#[path = "kernel_runtime/vfs_integration.rs"]
pub mod vfs_integration;

#[cfg(all(feature = "drivers", feature = "networking"))]
#[path = "kernel_runtime/network_policy_helpers/mod.rs"]
mod network_policy_helpers;

#[cfg(all(feature = "drivers", feature = "networking"))]
#[path = "kernel_runtime/network_remediation/mod.rs"]
mod network_remediation;

// ── KernelRuntime ─────────────────────────────────────────────────────────────

pub struct KernelRuntime {
    /// Per-CPU IRQ dispatcher, x86_64 only.
    #[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
    dispatcher: ActiveDispatcher,
}

pub(crate) fn heap_ready() -> bool {
    heap::heap_ready()
}

impl KernelRuntime {
    pub fn new() -> Self {
        Self {
            #[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
            dispatcher: ActiveDispatcher::new(),
        }
    }

    /// Main entry point.  Runs the full boot sequence then enters the main loop.
    pub fn run(self) -> ! {
        Hal::early_init();
        // Diagnostics: write an explicit early-serial marker immediately after HAL early init
        Hal::serial_write_raw("[EARLY SERIAL] post Hal::early_init\n");
        Hal::serial_write_raw("[BOOT] AetherXOS Elite Runtime activation start\n");

        let orchestrator = &crate::kernel::boot_graph::ORCHESTRATOR;

        // Initialize system heap early so subsequent logging and subsystem
        // registration (which may allocate) can safely use the allocator.
        #[cfg(target_os = "none")]
        {
            Hal::serial_write_raw("[EARLY SERIAL] initializing system heap\n");
            heap::init_heap(&crate::ALLOCATOR);
            Hal::serial_write_raw("[EARLY SERIAL] system heap initialized\n");
        }

        // 1. Initial handoff
        orchestrator.reach(BootStage::BootloaderHandoff).expect("Failed to reach BootloaderHandoff");

        let boot = runtime_boot::RuntimeBootContext::start();

        orchestrator.reach(BootStage::EarlyMemory).expect("Failed to reach EarlyMemory");
        
        Hal::serial_write_raw("[EARLY SERIAL] before tty init and after_heap_init\n");
        aethercore::kernel::tty::init_default_tty();
        Hal::serial_write_raw("[EARLY SERIAL] before boot.after_heap_init\n");
        boot.after_heap_init();
        Hal::serial_write_raw("[EARLY SERIAL] after boot.after_heap_init\n");
        Hal::serial_write_raw("[EARLY SERIAL] before boot.after_hal_early_init\n");
        boot.after_hal_early_init();
        Hal::serial_write_raw("[EARLY SERIAL] after boot.after_hal_early_init\n");
        Hal::serial_write_raw("[EARLY SERIAL] before boot.assert_self_tests\n");
        boot.assert_self_tests();
        Hal::serial_write_raw("[EARLY SERIAL] after boot.assert_self_tests\n");

        // 3. Platform services
        Hal::serial_write_raw("[EARLY SERIAL] reaching PlatformEarly\n");
        orchestrator.reach(BootStage::PlatformEarly).expect("Failed to reach PlatformEarly");
        Hal::serial_write_raw("[EARLY SERIAL] reached PlatformEarly\n");

        self.init_platform_services();
        boot.after_platform_services();

        // 4. Device discovery
        Hal::serial_write_raw("[EARLY SERIAL] reaching PlatformDevices\n");
        orchestrator.reach(BootStage::PlatformDevices).expect("Failed to reach PlatformDevices");
        Hal::serial_write_raw("[EARLY SERIAL] reached PlatformDevices\n");

        // 5. Core subsystems activation
        self.run_runtime_activation();
        Hal::serial_write_raw("[EARLY SERIAL] reaching CoreSubsystems\n");
        orchestrator.reach(BootStage::CoreSubsystems).expect("Failed to reach CoreSubsystems");
        Hal::serial_write_raw("[EARLY SERIAL] reached CoreSubsystems\n");

        // 6. Finalization & Interrupts
        self.finalize_runtime_activation();
        Hal::serial_write_raw("[EARLY SERIAL] reaching UserspaceReady\n");
        orchestrator.reach(BootStage::UserspaceReady).expect("Failed to reach UserspaceReady");
        Hal::serial_write_raw("[EARLY SERIAL] reached UserspaceReady\n");
        
        log::info(&orchestrator.diagnostics());
        log::info("Kernel runtime fully operational. Entering main loop.");

        boot.enter_main_loop();
    }
}
