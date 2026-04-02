use super::{
    BootInfo, BootModule, MAX_BOOT_MODULES, MAX_KERNEL_CMDLINE_BYTES, MAX_USABLE_REGIONS, MemRegion,
};
use spin::Once;

static BOOT_INFO: Once<BootInfo> = Once::new();
static EMPTY_BOOT_INFO: BootInfo = BootInfo {
    hhdm_offset: 0,
    total_usable_bytes: 0,
    largest_region: MemRegion { base: 0, len: 0 },
    usable_regions: [MemRegion { base: 0, len: 0 }; MAX_USABLE_REGIONS],
    usable_region_count: 0,
    rsdp_phys: 0,
    dtb_phys: 0,
    kernel_cmdline: [0u8; MAX_KERNEL_CMDLINE_BYTES],
    modules: [BootModule {
        phys_base: 0,
        size: 0,
        cmdline: [0u8; 64],
    }; MAX_BOOT_MODULES],
    module_count: 0,
    framebuffer: None,
    total_map_bytes: 0,
    map_entry_count: 0,
};

pub fn init() {
    BOOT_INFO.call_once(super::collect);
}

#[inline(always)]
pub fn get() -> &'static BootInfo {
    BOOT_INFO.get().unwrap_or(&EMPTY_BOOT_INFO)
}

#[inline(always)]
pub fn try_get() -> Option<&'static BootInfo> {
    BOOT_INFO.get()
}
