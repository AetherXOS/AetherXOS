use super::{PlatformKind, PlatformStatus};

pub(super) fn status(
    acpi_present: bool,
    dtb_present: bool,
    virt: crate::hal::common::virt::VirtStatus,
    gic: crate::hal::aarch64::gic::GicStats,
    timer: crate::hal::aarch64::timer::GenericTimerStats,
    smp: crate::hal::aarch64::smp::PsciBootStats,
) -> PlatformStatus {
    let virt_status = super::support::virt_platform_status(virt, gic, timer, dtb_present);
    super::support::compose_platform_status(
        super::support::PlatformBaseStatus {
            kind: PlatformKind::Virt,
            acpi_present,
            dtb_present,
            hypervisor_present: virt.caps.hypervisor_present,
            gic_initialized: gic.initialized,
            cpu_count: super::super::smp::cpu_count(),
            aps_ready: smp.aps_ready,
            timer_frequency_hz: timer.frequency_hz,
            vm_launch_ready: virt.vm_launch_ready,
        },
        virt_status,
    )
}
