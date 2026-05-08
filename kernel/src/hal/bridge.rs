//! Bridge between `core` traits and the existing HAL implementation.
//!
//! Provides `CoreHal` which delegates to the current `HAL` type so callers can
//! depend on `core::traits::hardware::HardwareAbstraction` without touching
//! architecture-specific implementation details.

use crate::core::traits::hardware::*;

pub struct CoreHal;

impl HardwareAbstraction for CoreHal {
    fn enable_interrupts() {
        crate::hal::HAL::enable_interrupts();
    }

    fn disable_interrupts() {
        crate::hal::HAL::disable_interrupts();
    }

    fn irq_save() -> usize {
        crate::hal::HAL::irq_save()
    }

    fn irq_restore(flags: usize) {
        crate::hal::HAL::irq_restore(flags);
    }

    fn halt() {
        crate::hal::HAL::halt();
    }

    fn early_init() {
        crate::hal::HAL::early_init();
    }

    fn init_interrupts() {
        crate::hal::HAL::init_interrupts();
    }

    fn init_timer() {
        crate::hal::HAL::init_timer();
    }

    fn init_smp() {
        crate::hal::HAL::init_smp();
    }

    fn init_cpu_local(ptr: usize) {
        crate::hal::HAL::init_cpu_local(ptr);
    }

    fn set_performance_profile(profile: PerformanceProfile) {
        crate::hal::HAL::set_performance_profile(profile);
    }

    fn serial_write_raw(s: &str) {
        crate::hal::HAL::serial_write_raw(s);
    }

    fn panic_with_report(info: &core::panic::PanicInfo, report: &crate::kernel::CrashReport) -> ! {
        crate::hal::HAL::panic_with_report(info, report)
    }

    fn fatal_halt(reason: &str) -> ! {
        crate::hal::HAL::fatal_halt(reason)
    }

    fn idle_once() {
        crate::hal::HAL::idle_once();
    }

    fn get_time_ns() -> u64 {
        crate::hal::HAL::get_time_ns()
    }
}

pub const CORE_HAL: CoreHal = CoreHal;
