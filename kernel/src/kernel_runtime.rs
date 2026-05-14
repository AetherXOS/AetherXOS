extern crate alloc;
use alloc::format;

// ── Architecture imports ──────────────────────────────────────────────────────
use crate::core::log;
use aethercore::hal::Hal;

// ── Phase 6: Boot Infrastructure Integration ──────────────────────────────────
use crate::interfaces::boot::{BootManager, BootStage};
use crate::kernel::boot_manager::GLOBAL_BOOT_MANAGER;

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
        log::info("AetherXOS Elite Runtime activation start");

        let orchestrator = &crate::kernel::boot_graph::ORCHESTRATOR;

        // 1. Initial handoff
        orchestrator.reach(BootStage::BootloaderHandoff).expect("Failed to reach BootloaderHandoff");
        
        let boot = runtime_boot::RuntimeBootContext::start();
        
        // 2. Memory & Heap initialization
        orchestrator.reach(BootStage::EarlyMemory).expect("Failed to reach EarlyMemory");

        #[cfg(target_os = "none")]
        {
            log::info("Initializing system heap...");
            heap::init_heap(&crate::ALLOCATOR);
            log::info("System heap initialized");
        }
        
        aethercore::kernel::tty::init_default_tty();
        boot.after_heap_init();
        boot.after_hal_early_init();
        boot.assert_self_tests();

        // 3. Platform services
        orchestrator.reach(BootStage::PlatformEarly).expect("Failed to reach PlatformEarly");
        
        self.init_platform_services();
        boot.after_platform_services();

        // 4. Device discovery
        orchestrator.reach(BootStage::PlatformDevices).expect("Failed to reach PlatformDevices");

        // 5. Core subsystems activation
        self.run_runtime_activation();
        orchestrator.reach(BootStage::CoreSubsystems).expect("Failed to reach CoreSubsystems");

        // 6. Finalization & Interrupts
        self.finalize_runtime_activation();
        orchestrator.reach(BootStage::UserspaceReady).expect("Failed to reach UserspaceReady");
        
        log::info(&orchestrator.diagnostics());
        log::info("Kernel runtime fully operational. Entering main loop.");

        boot.enter_main_loop();
    }
}
