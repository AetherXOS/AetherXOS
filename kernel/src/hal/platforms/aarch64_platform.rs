// --- ARM64 (aarch64) PLATFORM IMPLEMENTATION ---
// CPU detection, device tree parsing, timing, and platform-specific services

use crate::core::log;
use crate::interfaces::platform::{CpuFeatures, MemoryLayout, Platform, PlatformCapabilities, PlatformServices};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// ARM64 CPU features detected at boot
#[derive(Debug, Clone, Copy)]
pub struct Aarch64Features {
    pub has_generic_timer: bool,
    pub has_gic: bool,
    pub has_sve: bool,
    pub has_pmu: bool,
    pub has_mte: bool,
    pub supports_smp: bool,
}

impl Aarch64Features {
    pub fn detect() -> Self {
        Self {
            has_generic_timer: true,
            has_gic: true,
            has_sve: false,
            has_pmu: true,
            has_mte: false,
            supports_smp: true,
        }
    }
}

/// ARM64 memory layout (typically from device tree)
#[derive(Debug, Clone, Copy)]
pub struct Aarch64MemoryLayout {
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub physical_memory_size: u64,
    pub direct_map_base: u64,
    pub virt_bias: u64,
}

impl Aarch64MemoryLayout {
    pub fn new(dtb_address: u64) -> Self {
        Self {
            kernel_base: 0xffff800000000000,
            kernel_size: 0x80000000,
            physical_memory_size: 0,
            direct_map_base: 0xffff800000000000,
            virt_bias: dtb_address,
        }
    }
}

/// Concrete ARM64 PlatformServices implementation
pub struct Aarch64PlatformServices {
    features: Aarch64Features,
    memory: Aarch64MemoryLayout,
    cpu_count: AtomicU64,
    is_running: AtomicBool,
}

impl Aarch64PlatformServices {
    pub fn new(dtb_address: u64) -> Self {
        Self {
            features: Aarch64Features::detect(),
            memory: Aarch64MemoryLayout::new(dtb_address),
            cpu_count: AtomicU64::new(1),
            is_running: AtomicBool::new(false),
        }
    }

    pub fn set_cpu_count(&self, count: u64) {
        self.cpu_count.store(count, Ordering::Release);
    }

    fn read_virtual_timer(&self) -> u64 {
        crate::hal::cpu::rdtsc()
    }

    fn get_cpu_id(&self) -> u32 {
        unsafe {
            let mut mpidr: u64;
            core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr);
            (mpidr & 0xff) as u32
        }
    }

    fn timer_frequency(&self) -> u64 {
        unsafe {
            let mut freq: u64;
            core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
            freq
        }
    }

    fn push_bytes(&self, buf: &mut [u8], off: &mut usize, bytes: &[u8]) -> Option<()> {
        let end = off.checked_add(bytes.len())?;
        if end > buf.len() {
            return None;
        }
        buf[*off..end].copy_from_slice(bytes);
        *off = end;
        Some(())
    }

    fn push_aarch64_mov_reg_imm64(&self, buf: &mut [u8], off: &mut usize, reg: u8, imm: u64) -> Option<()> {
        let mut insn = 0xd2800000 | ((reg as u32) & 0x1f) | (((imm & 0xffff) as u32) << 5);
        self.push_bytes(buf, off, &insn.to_le_bytes())?;
        insn = 0xf2a00000 | ((reg as u32) & 0x1f) | ((((imm >> 16) & 0xffff) as u32) << 5) | (1 << 21);
        self.push_bytes(buf, off, &insn.to_le_bytes())?;
        insn = 0xf2c00000 | ((reg as u32) & 0x1f) | ((((imm >> 32) & 0xffff) as u32) << 5) | (2 << 21);
        self.push_bytes(buf, off, &insn.to_le_bytes())?;
        insn = 0xf2e00000 | ((reg as u32) & 0x1f) | ((((imm >> 48) & 0xffff) as u32) << 5) | (3 << 21);
        self.push_bytes(buf, off, &insn.to_le_bytes())?;
        Some(())
    }
}

impl PlatformServices for Aarch64PlatformServices {
    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            architecture: "aarch64".into(),
            platform_name: "Generic ARM64".into(),
            cpu_count: self.cpu_count() as u64,
            has_smp: self.features.supports_smp,
            has_virtualization: true,
            has_smm: false,
            has_nested_paging: true,
            supports_cpuid: false,
            cpu_features: self.cpu_features(),
            memory_layout: self.memory_layout(),
            has_acpi: false,
            has_device_tree: true,
            max_interrupts: 1024,
        }
    }

    fn memory_layout(&self) -> MemoryLayout {
        MemoryLayout {
            kernel_base: self.memory.kernel_base,
            kernel_size: self.memory.kernel_size,
            total_memory: self.memory.physical_memory_size as usize,
            usable_memory: self.memory.physical_memory_size as usize,
            physical_memory_size: self.memory.physical_memory_size,
            direct_map_base: self.memory.direct_map_base,
            virt_bias: self.memory.virt_bias,
            reserved_start: 0,
            reserved_end: 0,
        }
    }

    fn cpu_features(&self) -> CpuFeatures {
        CpuFeatures {
            has_apic: self.features.has_gic,
            has_tsc: self.features.has_generic_timer,
            has_msr: true,
            has_paging: true,
            has_interrupts: true,
            has_virtualization: true,
            has_protection: true,
            supports_smp: self.features.supports_smp,
            supports_virtualization: true,
            cpu_count: self.cpu_count(),
            cpu_freq_mhz: 2000,
        }
    }

    fn current_cpu_id(&self) -> u32 { self.get_cpu_id() }
    fn cpu_count(&self) -> u32 { self.cpu_count.load(Ordering::Acquire) as u32 }
    fn halt_cpu(&self, _cpu_id: u32) {
        unsafe { loop { core::arch::aarch64::wfe(); } }
    }
    fn reset_platform(&self, _cold_reset: bool) {
        unsafe { loop { core::arch::aarch64::wfe(); } }
    }
    fn shutdown(&self) {
        unsafe { loop { core::arch::aarch64::wfe(); } }
    }
    fn cycle_count(&self) -> u64 { self.read_virtual_timer() }
    fn current_time_ns(&self) -> u64 {
        let ticks = self.read_virtual_timer();
        let freq = self.timer_frequency();
        if freq > 0 { (ticks * 1_000_000_000) / freq } else { ticks }
    }

    fn enable_interrupts(&self) {
        unsafe { core::arch::asm!("msr daifclr, #2", options(nomem, nostack)); }
    }

    fn disable_interrupts(&self) {
        unsafe { core::arch::asm!("msr daifset, #2", options(nomem, nostack)); }
    }

    fn interrupts_enabled(&self) -> bool {
        let daif: u64;
        unsafe {
            core::arch::asm!("mrs {}, daif", out(reg) daif);
        }
        (daif & (1 << 7)) == 0 // I-bit is inverted in sense (1 means masked)
    }

    fn flush_tlb(&self, addr: Option<u64>) {
        unsafe {
            if let Some(a) = addr {
                core::arch::asm!("tlbi vaae1is, {}", in(reg) (a >> 12), options(nostack));
            } else {
                core::arch::asm!("tlbi vmalle1is", options(nostack));
            }
            core::arch::asm!("dsb ish; isb", options(nostack));
        }
    }

    fn set_page_table(&self, root_phys_addr: u64) {
        unsafe {
            core::arch::asm!("msr ttbr0_el1, {}", in(reg) root_phys_addr, options(nostack));
            core::arch::asm!("isb", options(nostack));
        }
    }

    fn encode_init_trampoline(&self, buf: &mut [u8], hooks: &[u64], final_entry: u64) -> Option<usize> {
        let mut off = 0usize;
        for hook in hooks {
            self.push_aarch64_mov_reg_imm64(buf, &mut off, 0, *hook)?;
            self.push_bytes(buf, &mut off, &0xb4000040u32.to_le_bytes())?;
            self.push_bytes(buf, &mut off, &0xd63f0000u32.to_le_bytes())?;
        }
        self.push_aarch64_mov_reg_imm64(buf, &mut off, 0, final_entry)?;
        self.push_bytes(buf, &mut off, &0xd61f0000u32.to_le_bytes())?;
        Some(off)
    }

    fn encode_fini_trampoline(&self, buf: &mut [u8], hooks: &[u64]) -> Option<usize> {
        let mut off = 0usize;
        for hook in hooks {
            self.push_aarch64_mov_reg_imm64(buf, &mut off, 0, *hook)?;
            self.push_bytes(buf, &mut off, &0xb4000040u32.to_le_bytes())?;
            self.push_bytes(buf, &mut off, &0xd63f0000u32.to_le_bytes())?;
        }
        self.push_bytes(buf, &mut off, &0xd2800000u32.to_le_bytes())?;
        self.push_bytes(buf, &mut off, &0xd65f03c0u32.to_le_bytes())?;
        Some(off)
    }
}

pub struct Aarch64Platform {
    services: Aarch64PlatformServices,
    is_initialized: AtomicBool,
}

impl Aarch64Platform {
    pub fn new(dtb_address: u64) -> Self {
        Self {
            services: Aarch64PlatformServices::new(dtb_address),
            is_initialized: AtomicBool::new(false),
        }
    }
}

impl Platform for Aarch64Platform {
    fn init(&self) -> crate::interfaces::KernelResult<()> {
        self.is_initialized.store(true, Ordering::Release);
        Ok(())
    }
    fn is_ready(&self) -> bool { self.is_initialized.load(Ordering::Acquire) }
    fn shutdown_platform(&self) -> crate::interfaces::KernelResult<()> {
        self.services.shutdown();
        Ok(())
    }
    fn services(&self) -> &dyn PlatformServices { &self.services }
}

impl PlatformServices for Aarch64Platform {
    fn capabilities(&self) -> PlatformCapabilities { self.services.capabilities() }
    fn memory_layout(&self) -> MemoryLayout { self.services.memory_layout() }
    fn cpu_features(&self) -> CpuFeatures { self.services.cpu_features() }
    fn current_cpu_id(&self) -> u32 { self.services.current_cpu_id() }
    fn cpu_count(&self) -> u32 { self.services.cpu_count() }
    fn halt_cpu(&self, cpu_id: u32) { self.services.halt_cpu(cpu_id) }
    fn reset_platform(&self, cold_reset: bool) { self.services.reset_platform(cold_reset) }
    fn shutdown(&self) { self.services.shutdown() }
    fn current_time_ns(&self) -> u64 { self.services.current_time_ns() }
    fn cycle_count(&self) -> u64 { self.services.cycle_count() }
    fn enable_interrupts(&self) { self.services.enable_interrupts() }
    fn disable_interrupts(&self) { self.services.disable_interrupts() }
    fn interrupts_enabled(&self) -> bool { self.services.interrupts_enabled() }
    fn flush_tlb(&self, addr: Option<u64>) { self.services.flush_tlb(addr) }
    fn set_page_table(&self, root_phys_addr: u64) { self.services.set_page_table(root_phys_addr) }
    fn encode_init_trampoline(&self, buf: &mut [u8], hooks: &[u64], final_entry: u64) -> Option<usize> {
        self.services.encode_init_trampoline(buf, hooks, final_entry)
    }
    fn encode_fini_trampoline(&self, buf: &mut [u8], hooks: &[u64]) -> Option<usize> {
        self.services.encode_fini_trampoline(buf, hooks)
    }
}

pub static AARCH64_PLATFORM: Aarch64Platform = Aarch64Platform::new(0);
