use super::BootInfo;

#[cfg(target_arch = "x86_64")]
pub(super) fn collect_rsdp(info: &mut BootInfo) {
    info.rsdp_phys = hypercore::hal::x86_64::acpi_rsdp_addr().unwrap_or(0);
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn collect_rsdp(_info: &mut BootInfo) {}

pub(super) fn collect_dtb(info: &mut BootInfo) {
    #[cfg(target_arch = "x86_64")]
    {
        if let Some(addr) = hypercore::hal::x86_64::dtb_addr() {
            info.dtb_phys = addr;
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        if let Some(addr) = hypercore::hal::dtb_addr() {
            info.dtb_phys = addr;
        }
    }
}
