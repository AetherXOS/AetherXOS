#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unused_must_use)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
extern crate hypercore; // Use the library
mod kernel_runtime;

use core::panic::PanicInfo;

// Global Allocator Definition
use hypercore::modules::allocators::selector::ActiveHeapAllocator;

#[global_allocator]
pub static ALLOCATOR: ActiveHeapAllocator = ActiveHeapAllocator::new();

// 3. The Kernel Entry Point
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let kernel = kernel_runtime::KernelRuntime::new();
    kernel.run();
}

/// Panic Handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    hypercore::kernel::panic_report(info, "panic");
}

// Test Runner
pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}
