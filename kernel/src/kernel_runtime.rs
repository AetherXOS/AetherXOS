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
mod boot_integration;
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
        log::info("Kernel Runtime activation start");

        // ── Phase 6: Stage 1 - BootloaderHandoff ──────────────────────────────
        match GLOBAL_BOOT_MANAGER.enter_stage(BootStage::BootloaderHandoff) {
            Ok(_) => log::info("BootloaderHandoff stage entered"),
            Err(e) => {
                log::error(&format!("Failed to enter BootloaderHandoff: {:?}", e));
                loop {}
            }
        }

        let boot = runtime_boot::RuntimeBootContext::start();
        log::info("Runtime boot context successfully ready");

        // ── Phase 6: Stage 2 - EarlyMemory ────────────────────────────────────
        match GLOBAL_BOOT_MANAGER.enter_stage(BootStage::EarlyMemory) {
            Ok(_) => {
                log::info("EarlyMemory stage entered");
                if let Err(e) = boot_integration::register_boot_subsystems(BootStage::EarlyMemory) {
                    log::error(&format!(
                        "Failed to register EarlyMemory subsystems: {:?}",
                        e
                    ));
                    loop {}
                }
            }
            Err(e) => {
                log::error(&format!("Failed to enter EarlyMemory: {:?}", e));
                loop {}
            }
        }

        // 1. Heap  (uses bi.largest_region internally for x86_64)
        log::info("Initializing system heap...");
        #[cfg(target_os = "none")]
        {
            heap::init_heap(&crate::ALLOCATOR);
        }
        #[cfg(not(target_os = "none"))]
        {
            log::debug("Skipping heap init on non-bare-metal target");
        }
        log::info("System heap initialized");
        // 1.5 TTY
        aethercore::kernel::tty::init_default_tty();
        boot.after_heap_init();
        log::info("After-heap-init hook complete");

        boot.after_hal_early_init();
        log::trace("after_hal_early_init hook returned");

        // 2.5 Boot self-test / config guardrails
        boot.assert_self_tests();
        log::info("Internal consistency tests passed");

        // ── Phase 6: Stage 3 - PlatformEarly ──────────────────────────────────
        log::info("Initializing platform services...");
        match GLOBAL_BOOT_MANAGER.enter_stage(BootStage::PlatformEarly) {
            Ok(_) => {
                log::info("PlatformEarly stage entered");

                // Initialize platform (CPU detection, timing, etc.)
                if let Err(e) = boot_integration::initialize_platform() {
                    log::error(&format!("Failed to initialize platform: {:?}", e));
                    loop {}
                }

                // Register platform subsystems
                if let Err(e) = boot_integration::register_boot_subsystems(BootStage::PlatformEarly)
                {
                    log::error(&format!(
                        "Failed to register PlatformEarly subsystems: {:?}",
                        e
                    ));
                    loop {}
                }
            }
            Err(e) => {
                log::error(&format!("Failed to enter PlatformEarly: {:?}", e));
                loop {}
            }
        }

        self.init_platform_services();
        log::info("Platform services active");
        boot.after_platform_services();
        log::info("Platform services post-startup hook returned");

        // ── Phase 6: Stage 4 - PlatformDevices ────────────────────────────────
        match GLOBAL_BOOT_MANAGER.enter_stage(BootStage::PlatformDevices) {
            Ok(_) => {
                log::info("PlatformDevices stage entered");

                // Enumerate devices and populate device manager
                if let Err(e) = boot_integration::enumerate_devices() {
                    log::error(&format!("Failed to enumerate devices: {:?}", e));
                    loop {}
                }

                // Register device subsystems
                if let Err(e) =
                    boot_integration::register_boot_subsystems(BootStage::PlatformDevices)
                {
                    log::error(&format!(
                        "Failed to register PlatformDevices subsystems: {:?}",
                        e
                    ));
                    loop {}
                }
            }
            Err(e) => {
                log::error(&format!("Failed to enter PlatformDevices: {:?}", e));
                loop {}
            }
        }

        // 4-9. IRQ, VM, PCI/IOMMU, drivers, SMP, IDT and interrupt enable
        self.run_runtime_activation();
        log::info("Runtime core activation successful");

        // ── Phase 6: Stage 5 - CoreSubsystems ─────────────────────────────────
        match GLOBAL_BOOT_MANAGER.enter_stage(BootStage::CoreSubsystems) {
            Ok(_) => {
                log::info("CoreSubsystems stage entered");

                // Register remaining core subsystems
                if let Err(e) =
                    boot_integration::register_boot_subsystems(BootStage::CoreSubsystems)
                {
                    log::error(&format!("Failed to register CoreSubsystems: {:?}", e));
                    loop {}
                }

                // Initialize all runtime extension subsystems
                if let Err(e) = boot_integration::initialize_runtime_extensions() {
                    log::error(&format!("Failed to initialize runtime extensions: {:?}", e));
                    loop {}
                }

                // Verify all critical subsystems are ready
                if let Err(e) = boot_integration::verify_subsystem_readiness() {
                    log::error(&format!("Subsystem readiness check failed: {:?}", e));
                    loop {}
                }
            }
            Err(e) => {
                log::error(&format!("Failed to enter CoreSubsystems: {:?}", e));
                loop {}
            }
        }

        self.finalize_runtime_activation();
        log::info("Interrupt routing initialized");

        // ── Phase 6: Stage 6 - UserspaceReady ─────────────────────────────────
        match GLOBAL_BOOT_MANAGER.enter_stage(BootStage::UserspaceReady) {
            Ok(_) => {
                log::info("UserspaceReady stage entered");
                log::info(&boot_integration::get_boot_diagnostics());
            }
            Err(e) => {
                log::error(&format!("Failed to enter UserspaceReady: {:?}", e));
                loop {}
            }
        }

        boot.enter_main_loop();
    }
}
