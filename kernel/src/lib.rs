#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(all(test, target_os = "none"), feature(custom_test_frameworks))]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unused_must_use)]
#![allow(dead_code, unused_imports, unused_mut, unused_variables)]
#![allow(clippy::all)]
#![cfg_attr(all(test, target_os = "none"), test_runner(crate::test_runner))]
#![cfg_attr(all(test, target_os = "none"), reexport_test_harness_main = "test_main")]

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

#[cfg(all(test, target_os = "none"))]
pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}
