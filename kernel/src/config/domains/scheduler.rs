//! Domain-specific scheduler configuration.

use core::sync::atomic::{AtomicU64, Ordering};
use crate::generated_consts;

static TIME_SLICE_NS_OVERRIDE: AtomicU64 = AtomicU64::new(0);
static STACK_SIZE_PAGES_OVERRIDE: AtomicU64 = AtomicU64::new(0);

pub struct SchedulerConfig;

impl SchedulerConfig {
    pub fn time_slice_ns() -> u64 {
        let ov = TIME_SLICE_NS_OVERRIDE.load(Ordering::Relaxed);
        if ov != 0 { ov } else { generated_consts::TIME_SLICE_NS }
    }

    pub fn set_time_slice_ns(val: u64) {
        TIME_SLICE_NS_OVERRIDE.store(val, Ordering::Relaxed);
    }

    pub fn reset_time_slice_ns() {
        TIME_SLICE_NS_OVERRIDE.store(0, Ordering::Relaxed);
    }

    pub fn stack_size_pages() -> usize {
        let ov = STACK_SIZE_PAGES_OVERRIDE.load(Ordering::Relaxed);
        if ov != 0 { ov as usize } else { generated_consts::STACK_SIZE_PAGES }
    }

    pub fn set_stack_size_pages(val: usize) {
        STACK_SIZE_PAGES_OVERRIDE.store(val as u64, Ordering::Relaxed);
    }

    pub fn max_cpus() -> usize {
        generated_consts::KERNEL_MAX_CPUS
    }
}
