//! Boot Information Parser
//!
//! Provides a unified `BootInfo` snapshot gathered from the Limine bootloader
//! protocol. All parsing happens once at startup; consumers receive a
//! plain-data struct with no Limine types leaking out.
#![allow(dead_code)]

mod cmdline;
mod firmware;
mod framebuffer;
mod memory;
mod singleton;

use core::fmt;

pub use singleton::{get, init, try_get};

/// Maximum number of usable memory regions we report.
pub const MAX_USABLE_REGIONS: usize = 16;
/// Maximum number of Limine module entries we report.
pub const MAX_BOOT_MODULES: usize = 8;
/// Maximum number of kernel command-line bytes we retain.
pub const MAX_KERNEL_CMDLINE_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, Default)]
pub struct MemRegion {
    pub base: u64,
    pub len: u64,
}

impl MemRegion {
    #[inline(always)]
    pub fn end(&self) -> u64 {
        self.base.saturating_add(self.len)
    }

    #[inline(always)]
    pub fn contains_phys(&self, phys: u64) -> bool {
        phys >= self.base && phys < self.end()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BootModule {
    pub phys_base: u64,
    pub size: u64,
    pub cmdline: [u8; 64],
}

impl Default for BootModule {
    fn default() -> Self {
        Self {
            phys_base: 0,
            size: 0,
            cmdline: [0u8; 64],
        }
    }
}

impl BootModule {
    pub fn cmdline_str(&self) -> &str {
        let end = self.cmdline.iter().position(|&b| b == 0).unwrap_or(64);
        core::str::from_utf8(&self.cmdline[..end]).unwrap_or("<invalid>")
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FramebufferInfo {
    pub phys_addr: u64,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    pub hhdm_offset: u64,
    pub total_usable_bytes: u64,
    pub largest_region: MemRegion,
    pub usable_regions: [MemRegion; MAX_USABLE_REGIONS],
    pub usable_region_count: usize,
    pub rsdp_phys: u64,
    pub dtb_phys: u64,
    pub kernel_cmdline: [u8; MAX_KERNEL_CMDLINE_BYTES],
    pub modules: [BootModule; MAX_BOOT_MODULES],
    pub module_count: usize,
    pub framebuffer: Option<FramebufferInfo>,
    pub total_map_bytes: u64,
    pub map_entry_count: usize,
}

impl Default for BootInfo {
    fn default() -> Self {
        Self {
            hhdm_offset: 0,
            total_usable_bytes: 0,
            largest_region: MemRegion::default(),
            usable_regions: [MemRegion::default(); MAX_USABLE_REGIONS],
            usable_region_count: 0,
            rsdp_phys: 0,
            dtb_phys: 0,
            kernel_cmdline: [0u8; MAX_KERNEL_CMDLINE_BYTES],
            modules: [BootModule::default(); MAX_BOOT_MODULES],
            module_count: 0,
            framebuffer: None,
            total_map_bytes: 0,
            map_entry_count: 0,
        }
    }
}

impl BootInfo {
    #[inline(always)]
    fn kernel_cmdline_len(&self) -> usize {
        self.kernel_cmdline
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(MAX_KERNEL_CMDLINE_BYTES)
    }

    #[inline(always)]
    pub fn kernel_cmdline_bytes(&self) -> &[u8] {
        &self.kernel_cmdline[..self.kernel_cmdline_len()]
    }

    #[inline(always)]
    pub fn phys_to_virt(&self, phys: u64) -> u64 {
        phys.saturating_add(self.hhdm_offset)
    }

    pub fn find_module(&self, needle: &[u8]) -> Option<&BootModule> {
        self.modules[..self.module_count].iter().find(|module| {
            let end = module.cmdline.iter().position(|&b| b == 0).unwrap_or(64);
            module.cmdline[..end]
                .windows(needle.len())
                .any(|window| window == needle)
        })
    }

    pub fn find_initrd(&self) -> Option<&BootModule> {
        self.find_module(b"initrd")
            .or_else(|| self.find_module(b"ramdisk"))
    }

    pub fn kernel_cmdline_str(&self) -> &str {
        core::str::from_utf8(self.kernel_cmdline_bytes()).unwrap_or("<invalid>")
    }

    pub fn kernel_cmdline_contains(&self, needle: &[u8]) -> bool {
        if needle.is_empty() {
            return true;
        }
        self.kernel_cmdline_bytes()
            .windows(needle.len())
            .any(|window| window == needle)
    }

    pub fn usable_regions_above(&self, min_size: u64) -> usize {
        self.usable_regions[..self.usable_region_count]
            .iter()
            .filter(|region| region.len >= min_size)
            .count()
    }
}

impl fmt::Display for BootInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BootInfo {{ hhdm={:#x} total_usable={} MiB largest_region=[{:#x}..{:#010x}] \
             rsdp={:#x} dtb={:#x} modules={} fb={} map_entries={} cmdline=\"{}\" }}",
            self.hhdm_offset,
            self.total_usable_bytes / (1024 * 1024),
            self.largest_region.base,
            self.largest_region.end(),
            self.rsdp_phys,
            self.dtb_phys,
            self.module_count,
            self.framebuffer.is_some(),
            self.map_entry_count,
            self.kernel_cmdline_str(),
        )
    }
}

pub fn collect() -> BootInfo {
    let mut info = BootInfo::default();
    memory::collect_hhdm_offset(&mut info);
    memory::collect_memory_map(&mut info);
    firmware::collect_rsdp(&mut info);
    cmdline::collect_kernel_cmdline(&mut info);
    firmware::collect_dtb(&mut info);
    framebuffer::collect_framebuffer(&mut info);
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn kernel_cmdline_contains_matches_raw_bytes_without_utf8_roundtrip() {
        let mut info = BootInfo::default();
        let cmdline = b"foo AETHERCORE_RUN_LINKED_PROBE=1 bar";
        info.kernel_cmdline[..cmdline.len()].copy_from_slice(cmdline);
        info.kernel_cmdline[cmdline.len()] = 0;

        assert!(info.kernel_cmdline_contains(b"AETHERCORE_RUN_LINKED_PROBE=1"));
        assert!(!info.kernel_cmdline_contains(b"AETHERCORE_RUN_LINKED_PROBE=0"));
    }

    #[test_case]
    fn kernel_cmdline_bytes_stops_at_nul_and_preserves_raw_bytes() {
        let mut info = BootInfo::default();
        let cmdline = b"alpha beta";
        info.kernel_cmdline[..cmdline.len()].copy_from_slice(cmdline);
        info.kernel_cmdline[cmdline.len()] = 0;
        info.kernel_cmdline[cmdline.len() + 1] = b'Z';

        assert_eq!(info.kernel_cmdline_bytes(), cmdline);
        assert_eq!(info.kernel_cmdline_str(), "alpha beta");
    }
}
