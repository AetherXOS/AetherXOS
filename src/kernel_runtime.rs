extern crate alloc;

// ── Architecture imports ──────────────────────────────────────────────────────
use hypercore::hal::HAL;

// ── IRQ dispatcher (x86_64 only) ─────────────────────────────────────────────
#[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
use hypercore::modules::dispatcher::selector::ActiveDispatcher;

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
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] kernel runtime run start\n");
        let boot = runtime_boot::RuntimeBootContext::start();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] runtime boot context ready\n");

        // 1. Heap  (uses bi.largest_region internally for x86_64)
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init call begin\n");
        heap::init_heap(&crate::ALLOCATOR);
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init returned\n");
        boot.after_heap_init();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] after_heap_init hook returned\n");

        // 2. HAL early init (GDT/IDT stubs, serial, etc.)
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] hal early init call begin\n");
        HAL::early_init();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] hal early init returned\n");
        boot.after_hal_early_init();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] after_hal_early_init hook returned\n",
        );

        // 2.5 Boot self-test / config guardrails
        boot.assert_self_tests();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] boot self tests returned\n");

        // 3. Platform services (ACPI, IOMMU, virt, VFS, networking bridge…)
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] platform services call begin\n");
        self.init_platform_services();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] platform services returned\n");
        boot.after_platform_services();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] after_platform_services hook returned\n",
        );

        // 4-9. IRQ, VM, PCI/IOMMU, drivers, SMP, IDT and interrupt enable
        self.run_runtime_activation();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] runtime activation returned\n");
        self.finalize_runtime_activation();
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] runtime interrupt routing returned\n",
        );

        boot.enter_main_loop();
    }
}
