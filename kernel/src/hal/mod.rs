//! Hardware Abstraction Layer


//! This layer provides unified abstractions for all hardware components.
//! Architecture-specific implementations (x86_64, aarch64) are confined within
//! this module. Generic kernel code has no direct dependencies on cfg(target_arch).

// ── Core abstraction traits (platform-agnostic) ────────────────────────────
pub mod abstractions;
pub mod firmware_abstraction;

// ── Component abstraction layers ─────────────────────────────────────────────
pub mod cpu_abstraction;
pub mod irq_abstraction;
pub mod timer_abstraction;

// ── Generic HAL components ──────────────────────────────────────────────────
pub mod common;
pub mod cpu;
pub mod port;
pub mod serial;
pub mod bridge;
pub mod mmio;
pub mod devices;
pub mod platforms;

// ── Device tree and firmware parsers (architecture-specific) ────────────────
#[cfg(target_arch = "x86_64")]
pub mod acpi_parser;

#[cfg(target_arch = "aarch64")]
pub mod dtb_parser;
use crate::interfaces::HardwareAbstraction;
use crate::interfaces::platform::Platform;

pub struct Hal;

impl HardwareAbstraction for Hal {
    #[inline(always)]
    fn enable_interrupts() {
        HAL::enable_interrupts();
    }

    #[inline(always)]
    fn disable_interrupts() {
        HAL::disable_interrupts();
    }

    #[inline(always)]
    fn irq_save() -> usize {
        HAL::irq_save()
    }

    #[inline(always)]
    fn irq_restore(flags: usize) {
        HAL::irq_restore(flags);
    }

    #[inline(always)]
    fn halt() {
        HAL::halt();
    }

    fn early_init() {
        HAL::early_init();
    }

    fn init_interrupts() {
        HAL::init_interrupts();
    }

    fn init_timer() {
        HAL::init_timer();
    }

    fn init_smp() {
        HAL::init_smp();
    }

    fn init_cpu_local(ptr: usize) {
        HAL::init_cpu_local(ptr);
    }

    fn set_performance_profile(profile: crate::interfaces::PerformanceProfile) {
        HAL::set_performance_profile(profile);
    }

    fn serial_write_raw(s: &str) {
        crate::hal::serial::write_raw(s);
    }

    fn get_time_ns() -> u64 {
        HAL::get_time_ns()
    }

    #[inline(always)]
    fn panic_with_report(info: &core::panic::PanicInfo, report: &crate::kernel::CrashReport) -> ! {
        HAL::panic_with_report(info, report);
    }

    #[inline(always)]
    fn fatal_halt(reason: &str) -> ! {
        HAL::fatal_halt(reason);
    }

    #[inline(always)]
    fn idle_once() {
        HAL::idle_once();
    }
}

impl Hal {
    // Re-expose for calls that might not want to use the trait explicitly, but keep it consistent.
    pub fn early_init() {
        <Self as HardwareAbstraction>::early_init();
    }

    pub fn init_smp() {
        <Self as HardwareAbstraction>::init_smp();
    }

    pub fn init_interrupts() {
        <Self as HardwareAbstraction>::init_interrupts();
    }

    pub fn init_timer() {
        <Self as HardwareAbstraction>::init_timer();
    }

    pub fn serial_write_raw(s: &str) {
        <Self as HardwareAbstraction>::serial_write_raw(s);
    }

    pub fn platform() -> &'static dyn Platform {
        crate::hal::platforms::get_platform()
    }

    pub fn firmware_provider() -> &'static dyn crate::hal::firmware_abstraction::FirmwareProvider {
        crate::hal::firmware_abstraction::get_firmware_provider()
    }
}

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

pub mod iommu;
pub mod wait;
pub use crate::kernel::syscalls::syscalls_consts;

// Re-export specific architectures based on build target
#[cfg(target_arch = "x86_64")]
pub use x86_64::HAL;
#[cfg(target_arch = "x86_64")]
pub use x86_64::{
    acpi, acpi_rsdp_addr, apic, dtb_addr, framebuffer, gdt, hhdm_offset, idt, input, mem_map,
    paging, pci, pic, platform, smp, virt,
};

#[cfg(target_arch = "aarch64")]
pub use aarch64::HAL;
#[cfg(target_arch = "aarch64")]
pub use aarch64::{
    acpi, dtb_addr, exception, framebuffer, gic, hhdm_offset, mem_map, paging, pci, platform,
    pl011, smp, timer, virt,
};
