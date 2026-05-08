mod devices;
mod irq;
mod memory;
mod routing;

use crate::kernel_runtime::KernelRuntime;
use crate::core::log;

#[inline(always)]
fn linked_probe_boot_mode() -> bool {
    crate::kernel_runtime::boot_info::try_get()
        .map(|info| info.kernel_cmdline_contains(b"AETHERCORE_RUN_LINKED_PROBE=1"))
        .unwrap_or(false)
}

#[inline(always)]
fn finalize_runtime_interrupt_window(runtime: KernelRuntime) {
    log::trace("finalize runtime interrupt window begin");
    runtime.finalize_runtime_interrupt_routing();
    log::trace("finalize runtime interrupt window returned");
}

#[inline(always)]
fn finalize_runtime_interrupt_enablement() {
    use aethercore::kernel::startup::{StartupStage, mark_stage};

    #[cfg(target_arch = "x86_64")]
    mark_stage(StartupStage::IdtReady);
    log::trace("idt ready");

    if linked_probe_boot_mode() {
        log::trace("interrupts deferred for linked probe");
        return;
    }

    routing::enable_runtime_interrupts();
    mark_stage(StartupStage::InterruptsEnabled);
    log::trace("interrupts enabled");
}

impl KernelRuntime {
    pub(super) fn run_runtime_activation(&self) {
        use aethercore::kernel::startup::{StartupStage, mark_stage};

        log::trace("runtime activation begin");
        self.register_runtime_irq_handlers();
        self.init_virtual_memory_runtime();
        mark_stage(StartupStage::IrqHandlersRegistered);
        log::trace("irq and vm runtime ready");

        self.init_pci_and_driver_runtime();
        log::trace("pci and drivers runtime ready");

        self.init_smp();
        mark_stage(StartupStage::SmpInit);
        log::trace("smp runtime ready");
    }

    pub(super) fn finalize_runtime_activation(self) {
        log::trace("finalize runtime activation begin");
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
