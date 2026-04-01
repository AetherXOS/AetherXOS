use crate::kernel_runtime::boot_info;

pub(crate) struct BootPrelude {
    pub(super) linked_probe_boot: bool,
    pub(super) boot_info_collected: bool,
    pub(super) has_cmdline: bool,
    pub(super) has_framebuffer: bool,
}

impl BootPrelude {
    pub(crate) fn linked_probe_boot(&self) -> bool {
        self.linked_probe_boot
    }
}

pub(crate) fn initialize_boot_prelude() -> BootPrelude {
    use hypercore::kernel::startup::{mark_stage, StartupStage};

    use hypercore::hal::Hal;
    Hal::serial_write_raw("[EARLY SERIAL] prelude init begin\n");
    boot_info::init();
    Hal::serial_write_raw("[EARLY SERIAL] prelude boot_info init returned\n");
    let bi = boot_info::get();
    Hal::serial_write_raw("[EARLY SERIAL] prelude boot_info get returned\n");

    let kernel_cmdline = bi.kernel_cmdline_str();
    Hal::serial_write_raw("[EARLY SERIAL] prelude cmdline parsed\n");
    let linked_probe_boot = kernel_cmdline.contains("HYPERCORE_RUN_LINKED_PROBE=1");

    Hal::serial_write_raw("[EARLY SERIAL] kernel_runtime entered\n");

    mark_stage(StartupStage::BootInfoCollected);

    BootPrelude {
        linked_probe_boot,
        boot_info_collected: true,
        has_cmdline: !kernel_cmdline.is_empty(),
        has_framebuffer: bi.framebuffer.is_some(),
    }
}

pub(crate) fn finalize_boot_prelude(prelude: &BootPrelude) {
    if !prelude.boot_info_collected {
        return;
    }

    use hypercore::hal::Hal;
    Hal::serial_write_raw("[EARLY SERIAL] prelude finalize begin\n");

    Hal::serial_write_raw(
        "[EARLY SERIAL] prelude finalize boot_info get begin\n",
    );
    let bi = boot_info::get();
    Hal::serial_write_raw(
        "[EARLY SERIAL] prelude finalize boot_info get returned\n",
    );
    Hal::serial_write_raw(
        "[EARLY SERIAL] prelude finalize boot summary begin\n",
    );
    hypercore::klog_info!("Boot: {}", bi);
    Hal::serial_write_raw(
        "[EARLY SERIAL] prelude finalize boot summary returned\n",
    );

    let kernel_cmdline = bi.kernel_cmdline_str();
    if prelude.has_cmdline {
        Hal::serial_write_raw(
            "[EARLY SERIAL] prelude finalize overrides begin\n",
        );
        match hypercore::config::KernelConfig::apply_kernel_cmdline_overrides(kernel_cmdline) {
            Ok(applied) if applied != 0 => {
                hypercore::klog_info!(
                    "Boot config overrides applied: count={} cmdline=\"{}\"",
                    applied,
                    kernel_cmdline
                );
            }
            Ok(_) => {}
            Err(err) => {
                hypercore::klog_warn!(
                    "Boot config override rejected at token={} key={} cause={:?} raw={}",
                    err.index,
                    err.key.as_str(),
                    err.cause,
                    err.raw_entry.as_str()
                );
            }
        }
        Hal::serial_write_raw(
            "[EARLY SERIAL] prelude finalize overrides returned\n",
        );
    }

    if prelude.has_framebuffer {
        Hal::serial_write_raw(
            "[EARLY SERIAL] prelude finalize framebuffer begin\n",
        );
        if let Some(fb) = bi.framebuffer {
            hypercore::klog_info!(
                "Framebuffer: {}x{} bpp={} pitch={} phys={:#x}",
                fb.width,
                fb.height,
                fb.bpp,
                fb.pitch,
                fb.phys_addr
            );
        }
        Hal::serial_write_raw(
            "[EARLY SERIAL] prelude finalize framebuffer returned\n",
        );
    }

    Hal::serial_write_raw("[EARLY SERIAL] prelude finalize returned\n");
}

pub(crate) fn log_linked_probe_boot(prelude: &BootPrelude) {
    if prelude.linked_probe_boot {
        use hypercore::hal::Hal;
        Hal::serial_write_raw("[EARLY SERIAL] linked probe cmdline observed\n");
        let kernel_cmdline = boot_info::get().kernel_cmdline_str();
        hypercore::klog_info!(
            "[LINKED PROBE] probe boot requested cmdline=\"{}\"",
            kernel_cmdline
        );
    }
}

pub(crate) fn write_stage_serial_marker(marker: &str) {
    use hypercore::hal::Hal;
    Hal::serial_write_raw(marker);
    Hal::serial_write_raw("\n");
}
