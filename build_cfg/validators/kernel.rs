//! Kernel config validation — arch, time slice, stack, CPU limits.

use crate::build_cfg::config_types::KernelConfig;

const VALID_ARCHES: &[&str] = &["x86_64", "aarch64"];
const MIN_TIME_SLICE_NS: u64 = 100_000; // 100µs
const MAX_TIME_SLICE_NS: u64 = 1_000_000_000; // 1s
const MIN_STACK_PAGES: usize = 2;
const MAX_STACK_PAGES: usize = 256;
const MAX_CPUS: usize = 1024;
const MIN_INTERRUPT_STACK_PAGES: usize = 1;
const MAX_INTERRUPT_STACK_PAGES: usize = 64;

pub fn validate(c: &KernelConfig) -> Vec<String> {
    let mut e = Vec::new();

    if !VALID_ARCHES.contains(&c.arch.as_str()) {
        e.push(format!(
            "kernel.arch '{}' invalid, expected one of {:?}",
            c.arch, VALID_ARCHES
        ));
    }
    if c.time_slice_ns < MIN_TIME_SLICE_NS || c.time_slice_ns > MAX_TIME_SLICE_NS {
        e.push(format!(
            "kernel.time_slice_ns {} out of range [{}, {}]",
            c.time_slice_ns, MIN_TIME_SLICE_NS, MAX_TIME_SLICE_NS
        ));
    }
    if c.stack_size_pages < MIN_STACK_PAGES || c.stack_size_pages > MAX_STACK_PAGES {
        e.push(format!(
            "kernel.stack_size_pages {} out of range [{}, {}]",
            c.stack_size_pages, MIN_STACK_PAGES, MAX_STACK_PAGES
        ));
    }
    if c.max_cpus == 0 || c.max_cpus > MAX_CPUS {
        e.push(format!(
            "kernel.max_cpus {} out of range [1, {}]",
            c.max_cpus, MAX_CPUS
        ));
    }
    if c.interrupt_stack_size_pages < MIN_INTERRUPT_STACK_PAGES
        || c.interrupt_stack_size_pages > MAX_INTERRUPT_STACK_PAGES
    {
        e.push(format!(
            "kernel.interrupt_stack_size_pages {} out of range [{}, {}]",
            c.interrupt_stack_size_pages, MIN_INTERRUPT_STACK_PAGES, MAX_INTERRUPT_STACK_PAGES
        ));
    }

    e
}
