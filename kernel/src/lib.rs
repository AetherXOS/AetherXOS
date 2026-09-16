#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![feature(custom_test_frameworks)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_must_use)]
#![cfg_attr(not(test), deny(dead_code))]
#![cfg_attr(test, allow(dead_code))]
#![allow(unexpected_cfgs)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate aethercore_common;
extern crate self as aethercore;
extern crate alloc;

pub mod aop;
pub mod bsp;
pub mod config;
pub mod core;
pub mod generated_consts;
pub mod hal;
pub mod interfaces;
pub mod kernel;
pub mod kernel_runtime;
pub mod modules;
pub mod services;

#[cfg(all(test, target_os = "none"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[global_allocator]
#[cfg(target_os = "none")]
pub static ALLOCATOR: modules::allocators::selector::ActiveHeapAllocator =
    modules::allocators::selector::ActiveHeapAllocator::new();

pub fn test_runner(tests: &[&dyn Fn()]) {
    #[cfg(not(target_os = "none"))]
    host_test_runner(tests);

    #[cfg(target_os = "none")]
    {
        for test in tests {
            test();
        }
    }
}

#[cfg(not(target_os = "none"))]
fn host_test_runner(tests: &[&dyn Fn()]) {
    use std::collections::HashSet;

    let start = std::env::var("AETHER_TEST_START")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let end = std::env::var("AETHER_TEST_END")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(tests.len());
    let trace = std::env::var("AETHER_TEST_TRACE")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let skip: HashSet<usize> = std::env::var("AETHER_TEST_SKIP")
        .ok()
        .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
        .unwrap_or_default();

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    for (idx, test) in tests.iter().enumerate() {
        if idx < start || idx >= end {
            continue;
        }
        if skip.contains(&idx) {
            if trace {
                eprintln!("[test_runner] idx={idx} - SKIPPED");
            }
            skipped += 1;
            continue;
        }
        if trace {
            eprintln!("[test_runner] idx={idx} - running...");
        }
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
        match res {
            Ok(()) => {
                if trace {
                    eprintln!("[test_runner] idx={idx} - PASSED");
                }
                passed += 1;
            }
            Err(_) => {
                eprintln!("[test_runner] idx={idx} - FAILED");
                failed += 1;
            }
        }
    }

    let total = passed + failed + skipped;
    eprintln!("[test_runner] Result: {passed} passed, {failed} failed, {skipped} skipped out of {total} run");
}
