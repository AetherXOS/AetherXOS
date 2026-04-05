#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![feature(custom_test_frameworks)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unused_must_use)]
#![allow(clippy::all)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate aethercore_common;

extern crate alloc;

// Publicly expose modules for testing and external usage (LibOS)
pub mod config;
pub mod generated_consts;
pub mod hal;
pub mod interfaces;
pub mod kernel;
pub mod modules;

#[cfg(all(test, target_os = "none"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(all(test, target_os = "none"))]
#[global_allocator]
static ALLOCATOR: modules::allocators::selector::ActiveHeapAllocator =
    modules::allocators::selector::ActiveHeapAllocator::new();

pub fn test_runner(tests: &[&dyn Fn()]) {
    #[cfg(all(not(target_os = "none"), windows))]
    {
        let _ = tests;
    }

        #[cfg(all(not(target_os = "none"), not(windows)))]
    {
        let start = std::env::var("AETHER_TEST_START")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let end = std::env::var("AETHER_TEST_END")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(tests.len());
        let trace = std::env::var("AETHER_TEST_TRACE")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        for (idx, test) in tests.iter().enumerate() {
            if idx < start || idx >= end {
                continue;
            }
            if trace {
                eprintln!("[test_runner] idx={idx}");
            }
            test();
        }
    }

    #[cfg(target_os = "none")]
    {
        for test in tests {
            test();
        }
    }
}
