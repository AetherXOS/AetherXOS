// --- x86_64 PLATFORM IMPLEMENTATION ---
// CPU discovery, memory layout, timing, and platform-specific services

use crate::core::log;
use crate::interfaces::platform::{CpuFeatures, MemoryLayout, Platform, PlatformCapabilities, PlatformServices};
use core::arch::x86_64::__cpuid;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// x86_64 CPU features detected at boot
#[derive(Debug, Clone, Copy)]
pub struct X86_64Features {
    pub has_apic: bool,
    pub has_tsc: bool,
    pub has_tsc_deadline: bool,
    pub has_msr: bool,
    pub has_pse: bool,
    pub has_pge: bool,
    pub has_pat: bool,
    pub has_mtrr: bool,
    pub supports_smp: bool,
}

impl X86_64Features {
    /// Detect CPU features at boot
    pub fn detect() -> Self {
        unsafe {
            // CPUID leaf 1: basic features
            let leaf1 = __cpuid(1);
            let has_apic = (leaf1.edx & (1 << 9)) != 0;
            let has_tsc = (leaf1.edx & (1 << 4)) != 0;
            let has_msr = (leaf1.edx & (1 << 5)) != 0;
            let has_pse = (leaf1.edx & (1 << 3)) != 0;
            let has_pge = (leaf1.edx & (1 << 13)) != 0;
            let has_pat = (leaf1.edx & (1 << 16)) != 0;
            let has_mtrr = (leaf1.edx & (1 << 12)) != 0;

            // CPUID leaf 0x80000001: extended features
            let leaf_ext = __cpuid(0x80000001);
            let supports_smp = (leaf_ext.edx & (1 << 27)) != 0; // RDTSCP support indicates SMP

            // CPUID leaf 0x00000001 ecx: extended features
            let has_tsc_deadline = (leaf1.ecx & (1 << 24)) != 0; // TSC-Deadline Timer

            Self {
                has_apic,
                has_tsc,
                has_tsc_deadline,
                has_msr,
                has_pse,
                has_pge,
                has_pat,
                has_mtrr,
                supports_smp,
            }
        }
    }
}

/// x86_64 memory layout information
#[derive(Debug, Clone, Copy)]
pub struct X86_64MemoryLayout {
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub physical_memory_size: u64,
    pub direct_map_base: u64,
    pub virt_bias: u64,
}

impl X86_64MemoryLayout {
    /// Get standard x86_64 memory layout
    pub fn new(hhdm_offset: u64) -> Self {
        Self {
            kernel_base: 0xffffffff80000000,
            kernel_size: 0x80000000,
            physical_memory_size: 0,
            direct_map_base: 0xffff800000000000,
            virt_bias: hhdm_offset,
        }
    }
}

/// Concrete x86_64 PlatformServices implementation
pub struct X86_64PlatformServices {
    features: X86_64Features,
    memory: X86_64MemoryLayout,
    cpu_count: AtomicU64,
    is_running: AtomicBool,
}

impl X86_64PlatformServices {
    /// Initialize platform services
    pub const fn new(hhdm_offset: u64) -> Self {
        Self {
            features: X86_64Features {
                has_apic: false,
                has_tsc: false,
                has_tsc_deadline: false,
                has_msr: false,
                has_pse: false,
                has_pge: false,
                has_pat: false,
                has_mtrr: false,
                supports_smp: false,
            },
            memory: X86_64MemoryLayout {
                kernel_base: 0xffffffff80000000,
                kernel_size: 0x80000000,
                physical_memory_size: 0,
                direct_map_base: 0xffff800000000000,
                virt_bias: hhdm_offset,
            },
            cpu_count: AtomicU64::new(1),
            is_running: AtomicBool::new(false),
        }
    }

    /// Perform hardware detection
    pub fn detect_hardware(&mut self) {
        self.features = X86_64Features::detect();
        log::info("Initializing x86_64 Platform Services");
    }

    /// Set the number of CPUs (called during SMP detection)
    pub fn set_cpu_count(&self, count: u64) {
        self.cpu_count.store(count, Ordering::Release);
    }

    /// Get CPU cycle counter (TSC-based)
    fn read_tsc(&self) -> u64 {
        if !self.features.has_tsc {
            return 0;
        }
        crate::hal::cpu::rdtsc()
    }

    /// Get CPU index (via CPUID)
    fn get_cpu_id(&self) -> u32 {
        0 // Placeholder
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
}

impl PlatformServices for X86_64PlatformServices {
    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            architecture: "x86_64".into(),
            platform_name: "Generic x86_64 PC".into(),
            cpu_count: self.cpu_count() as u64,
            has_smp: self.features.supports_smp,
            has_virtualization: true,
            has_smm: true,
            has_nested_paging: true,
            supports_cpuid: true,
            cpu_features: self.cpu_features(),
            memory_layout: self.memory_layout(),
            has_acpi: true,
            has_device_tree: false,
            max_interrupts: 256,
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
            has_apic: self.features.has_apic,
            has_tsc: self.features.has_tsc,
            has_msr: self.features.has_msr,
            has_paging: true,
            has_interrupts: true,
            has_virtualization: true,
            has_protection: true,
            supports_smp: self.features.supports_smp,
            supports_virtualization: true,
            cpu_count: self.cpu_count(),
            cpu_freq_mhz: 3000,
        }
    }

    fn current_cpu_id(&self) -> u32 { self.get_cpu_id() }
    fn cpu_count(&self) -> u32 { self.cpu_count.load(Ordering::Acquire) as u32 }
    fn halt_cpu(&self, _cpu_id: u32) {
        unsafe { core::arch::asm!("hlt"); }
    }
    fn reset_platform(&self, _cold_reset: bool) {
        unsafe { 
           use crate::interfaces::PortIo;
           crate::hal::x86_64::port::X86PortIo::outb(0x64, 0xfe); 
        }
    }
    fn shutdown(&self) {
        unsafe { loop { core::arch::asm!("hlt"); } }
    }
    fn cycle_count(&self) -> u64 { self.read_tsc() }
    fn current_time_ns(&self) -> u64 { self.read_tsc() }

    fn enable_interrupts(&self) {
        unsafe { core::arch::asm!("sti", options(nomem, nostack)); }
    }

    fn disable_interrupts(&self) {
        unsafe { core::arch::asm!("cli", options(nomem, nostack)); }
    }

    fn interrupts_enabled(&self) -> bool {
        let rflags: u64;
        unsafe {
            core::arch::asm!("pushfq; pop {}", out(reg) rflags);
        }
        (rflags & (1 << 9)) != 0
    }

    fn flush_tlb(&self, addr: Option<u64>) {
        unsafe {
            if let Some(a) = addr {
                core::arch::asm!("invlpg [{}]", in(reg) a, options(nostack, preserves_flags));
            } else {
                core::arch::asm!("mov rax, cr3; mov cr3, rax", out("rax") _, options(nostack, preserves_flags));
            }
        }
    }

    fn set_page_table(&self, root_phys_addr: u64) {
        unsafe {
            core::arch::asm!("mov cr3, {}", in(reg) root_phys_addr, options(nostack, preserves_flags));
        }
    }

    fn encode_init_trampoline(&self, buf: &mut [u8], hooks: &[u64], final_entry: u64) -> Option<usize> {
        let mut off = 0usize;
        for hook in hooks {
            self.push_bytes(buf, &mut off, &[0x48, 0xB8])?;
            self.push_bytes(buf, &mut off, &hook.to_le_bytes())?;
            self.push_bytes(buf, &mut off, &[0x48, 0x85, 0xC0])?;
            self.push_bytes(buf, &mut off, &[0x74, 0x02])?;
            self.push_bytes(buf, &mut off, &[0xFF, 0xD0])?;
        }
        self.push_bytes(buf, &mut off, &[0x48, 0xB8])?;
        self.push_bytes(buf, &mut off, &final_entry.to_le_bytes())?;
        self.push_bytes(buf, &mut off, &[0xFF, 0xE0])?;
        Some(off)
    }

    fn encode_fini_trampoline(&self, buf: &mut [u8], hooks: &[u64]) -> Option<usize> {
        let mut off = 0usize;
        for hook in hooks {
            self.push_bytes(buf, &mut off, &[0x48, 0xB8])?;
            self.push_bytes(buf, &mut off, &hook.to_le_bytes())?;
            self.push_bytes(buf, &mut off, &[0x48, 0x85, 0xC0])?;
            self.push_bytes(buf, &mut off, &[0x74, 0x02])?;
            self.push_bytes(buf, &mut off, &[0xFF, 0xD0])?;
        }
        self.push_bytes(buf, &mut off, &[0x31, 0xC0])?;
        self.push_bytes(buf, &mut off, &[0xC3])?;
        Some(off)
    }
}

pub struct X86_64Platform {
    services: X86_64PlatformServices,
    is_initialized: AtomicBool,
}

impl X86_64Platform {
    pub const fn new(hhdm_offset: u64) -> Self {
        Self {
            services: X86_64PlatformServices::new(hhdm_offset),
            is_initialized: AtomicBool::new(false),
        }
    }
    pub unsafe fn bootstrap_detection(&mut self) {
        self.services.detect_hardware();
    }
}

impl Platform for X86_64Platform {
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

impl PlatformServices for X86_64Platform {
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

pub static X86_64_PLATFORM: X86_64Platform = X86_64Platform::new(0xffff800000000000);
