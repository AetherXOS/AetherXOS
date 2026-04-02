mod devices;
mod irq;
mod memory;
mod routing;

use crate::kernel_runtime::KernelRuntime;

#[inline(always)]
fn linked_probe_boot_mode() -> bool {
    crate::kernel_runtime::boot_info::try_get()
        .map(|info| info.kernel_cmdline_contains(b"AETHERCORE_RUN_LINKED_PROBE=1"))
        .unwrap_or(false)
}

#[inline(always)]
fn finalize_runtime_interrupt_window(runtime: KernelRuntime) {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] finalize runtime interrupt window begin\n");
    runtime.finalize_runtime_interrupt_routing();
    aethercore::hal::serial::write_raw(
        "[EARLY SERIAL] finalize runtime interrupt window returned\n",
    );
}

#[inline(always)]
fn finalize_runtime_interrupt_enablement() {
    use aethercore::kernel::startup::{StartupStage, mark_stage};

    #[cfg(target_arch = "x86_64")]
    mark_stage(StartupStage::IdtReady);
    aethercore::hal::serial::write_raw("[EARLY SERIAL] idt ready\n");

    if linked_probe_boot_mode() {
        aethercore::hal::serial::write_raw("[EARLY SERIAL] interrupts deferred for linked probe\n");
        return;
    }

    routing::enable_runtime_interrupts();
    mark_stage(StartupStage::InterruptsEnabled);
    aethercore::hal::serial::write_raw("[EARLY SERIAL] interrupts enabled\n");
}

impl KernelRuntime {
    pub(super) fn run_runtime_activation(&self) {
        use aethercore::kernel::startup::{StartupStage, mark_stage};

        aethercore::hal::serial::write_raw("[EARLY SERIAL] runtime activation begin\n");
        self.register_runtime_irq_handlers();
        self.init_virtual_memory_runtime();
        mark_stage(StartupStage::IrqHandlersRegistered);
        aethercore::hal::serial::write_raw("[EARLY SERIAL] irq and vm runtime ready\n");

        self.init_pci_and_driver_runtime();
        aethercore::hal::serial::write_raw("[EARLY SERIAL] pci and drivers runtime ready\n");

        self.init_smp();
        mark_stage(StartupStage::SmpInit);
        aethercore::hal::serial::write_raw("[EARLY SERIAL] smp runtime ready\n");
    }

    pub(super) fn finalize_runtime_activation(self) {
        aethercore::hal::serial::write_raw("[EARLY SERIAL] finalize runtime activation begin\n");
        finalize_runtime_interrupt_window(self);
        finalize_runtime_interrupt_enablement();
    }
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn finalize_runtime_interrupt_window_is_callable() {
        let _f: fn(crate::kernel_runtime::KernelRuntime) = super::finalize_runtime_interrupt_window;
    }

    #[test_case]
    fn finalize_runtime_interrupt_enablement_is_callable() {
        super::finalize_runtime_interrupt_enablement();
    }
}
