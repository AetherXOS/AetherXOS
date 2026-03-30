use crate::kernel_runtime::boot_sequence::{self, BootPrelude};

pub(super) struct RuntimeBootContext {
    prelude: BootPrelude,
}

impl RuntimeBootContext {
    pub(super) fn start() -> Self {
        use hypercore::kernel::startup::{mark_stage, StartupStage};

        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] runtime boot start");
        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] runtime boot mark_stage begin");
        mark_stage(StartupStage::BootStart);
        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] runtime boot mark_stage returned");
        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] runtime boot prelude call begin");
        let prelude = boot_sequence::initialize_boot_prelude();
        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] runtime boot prelude returned");
        Self { prelude }
    }

    pub(super) fn after_heap_init(&self) {
        use hypercore::kernel::startup::{mark_stage, StartupStage};

        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] runtime boot heap init complete");
        mark_stage(StartupStage::HeapInit);
    }

    pub(super) fn after_hal_early_init(&self) {
        use hypercore::kernel::startup::{mark_stage, StartupStage};

        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] prelude finalize deferred");
        let _ = self.prelude.linked_probe_boot();
        boot_sequence::log_linked_probe_boot(&self.prelude);
        boot_sequence::write_stage_serial_marker(
            "[EARLY SERIAL] runtime boot hal early init complete",
        );
        mark_stage(StartupStage::HalEarlyInit);
    }

    pub(super) fn assert_self_tests(&self) {
        boot_sequence::assert_boot_self_tests();
    }

    pub(super) fn after_platform_services(&self) {
        use hypercore::kernel::startup::{mark_stage, StartupStage};

        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] prelude finalize begin");
        boot_sequence::finalize_boot_prelude(&self.prelude);
        boot_sequence::write_stage_serial_marker("[EARLY SERIAL] prelude finalize returned");
        crate::kernel_runtime::heap::finalize_heap_bootstrap();
        boot_sequence::write_stage_serial_marker(
            "[EARLY SERIAL] runtime boot platform services complete",
        );
        mark_stage(StartupStage::PlatformServicesInit);
    }

    pub(super) fn enter_main_loop(self) -> ! {
        prepare_runtime_main_loop_handoff();
        crate::kernel_runtime::main_loop::runtime_main_loop();
    }
}

#[inline(always)]
fn prepare_runtime_main_loop_handoff() {
    use hypercore::kernel::startup::{mark_stage, StartupStage};

    boot_sequence::log_boot_diagnostics();
    boot_sequence::write_stage_serial_marker("[EARLY SERIAL] runtime boot entering main loop");
    mark_stage(StartupStage::MainLoopEntered);
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn runtime_main_loop_handoff_helper_is_callable() {
        super::prepare_runtime_main_loop_handoff();
    }
}
