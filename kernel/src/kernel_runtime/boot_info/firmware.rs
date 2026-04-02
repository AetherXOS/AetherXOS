use super::BootInfo;

pub(super) fn collect_rsdp(info: &mut BootInfo) {
    #[cfg(target_arch = "x86_64")]
    {
        info.rsdp_phys = aethercore::hal::x86_64::acpi_rsdp_addr().unwrap_or(0);
    }
}

pub(super) fn collect_dtb(info: &mut BootInfo) {
    #[cfg(target_arch = "x86_64")]
    {
        if let Some(addr) = aethercore::hal::x86_64::dtb_addr() {
            info.dtb_phys = addr;
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        if let Some(addr) = aethercore::hal::dtb_addr() {
            info.dtb_phys = addr;
        }
    }
}
