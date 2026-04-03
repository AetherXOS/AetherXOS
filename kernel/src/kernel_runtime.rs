extern crate alloc;

// ── Architecture imports ──────────────────────────────────────────────────────
use aethercore::hal::{HAL, Hal};

// ── IRQ dispatcher (x86_64 only) ─────────────────────────────────────────────
#[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
use aethercore::modules::dispatcher::selector::ActiveDispatcher;

// ── Sub-modules ───────────────────────────────────────────────────────────────
#[path = "kernel_runtime/boot_flow/mod.rs"]
mod boot_flow;
#[path = "kernel_runtime/boot_info/mod.rs"]
mod boot_info;
#[path = "kernel_runtime/boot_sequence/mod.rs"]
mod boot_sequence;
#[path = "kernel_runtime/drivers_init/mod.rs"]
mod drivers_init;
#[path = "kernel_runtime/heap.rs"]
mod heap;
#[path = "kernel_runtime/interrupts/mod.rs"]
mod interrupts;
#[path = "kernel_runtime/main_loop.rs"]
mod main_loop;
#[path = "kernel_runtime/networking/mod.rs"]
mod networking;
#[path = "kernel_runtime/platform.rs"]
mod platform;
#[path = "kernel_runtime/platform_support/mod.rs"]
mod platform_support;
#[path = "kernel_runtime/runtime_boot.rs"]
mod runtime_boot;

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
        Hal::serial_write_raw("[BOOT] Kernel Runtime activation start\n");
        let boot = runtime_boot::RuntimeBootContext::start();
        Hal::serial_write_raw("[BOOT] Runtime boot context successfully ready\n");

        // 1. Heap  (uses bi.largest_region internally for x86_64)
        Hal::serial_write_raw("[BOOT] Initializing system heap...\n");
        heap::init_heap(&crate::ALLOCATOR);
        Hal::serial_write_raw("[BOOT] System heap initialized\n");
        // 1.5 TTY
        aethercore::kernel::tty::init_default_tty();
        boot.after_heap_init();
        Hal::serial_write_raw("[BOOT] After-heap-init hook complete\n");

        boot.after_hal_early_init();
        #[cfg(target_arch = "x86_64")]
        aethercore::hal::serial::write_raw(
            "[EARLY SERIAL] after_hal_early_init hook returned\n",
        );

        // 2.5 Boot self-test / config guardrails
        boot.assert_self_tests();
        Hal::serial_write_raw("[BOOT] Internal consistency tests passed\n");

        Hal::serial_write_raw("[BOOT] Initializing platform services...\n");
        self.init_platform_services();
        Hal::serial_write_raw("[BOOT] Platform services active\n");
        boot.after_platform_services();
        Hal::serial_write_raw("[BOOT] Platform services post-startup hook returned\n");

        // 4-9. IRQ, VM, PCI/IOMMU, drivers, SMP, IDT and interrupt enable
        self.run_runtime_activation();
        Hal::serial_write_raw("[BOOT] Runtime core activation successful\n");
        self.finalize_runtime_activation();
        Hal::serial_write_raw("[BOOT] Interrupt routing initialized\n");

        boot.enter_main_loop();
    }
}
