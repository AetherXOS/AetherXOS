#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![feature(custom_test_frameworks)]
#![cfg_attr(target_os = "none", feature(abi_x86_interrupt))]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unused_must_use)]
#![allow(dead_code, unused_imports, unused_mut, unused_variables)]
#![allow(clippy::all)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate aethercore;
extern crate alloc; // Use the library
#[cfg(target_os = "none")]
mod kernel_runtime;

#[cfg(target_os = "none")]
use core::panic::PanicInfo;

// Global Allocator Definition
use aethercore::modules::allocators::selector::ActiveHeapAllocator;

#[global_allocator]
#[cfg(target_os = "none")]
pub static ALLOCATOR: ActiveHeapAllocator = ActiveHeapAllocator::new();

// ============================================================================
// Multiboot2 Header - Required for QEMU x86_64 boot
// ============================================================================
// MUST be placed in first 32KB of binary, 8-byte aligned, and BEFORE entry point
#[repr(C, align(8))]
pub struct MultibootHeader {
    // Multiboot2 header
    magic: u32,         // 0xE85250D6 (magic number)
    architecture: u32,  // 0 = i386, 4 = x86_64
    header_length: u32, // 12 bytes (header + architecture + reserved) before tags
    checksum: u32,      // -(magic + architecture + header_length)

    // End tag (required)
    end_tag_type: u16,  // 0 = end tag
    end_tag_flags: u16, // 0
    end_tag_size: u32,  // 8 bytes
}

impl MultibootHeader {
    const fn new() -> Self {
        let magic = 0xE85250D6u32;
        let architecture = 0u32; // i386
        let header_length = 12u32; // Through checksum field
        let checksum = (0u32)
            .wrapping_sub(magic)
            .wrapping_sub(architecture)
            .wrapping_sub(header_length);

        MultibootHeader {
            magic,
            architecture,
            header_length,
            checksum,
            end_tag_type: 0,
            end_tag_flags: 0,
            end_tag_size: 8,
        }
    }
}

#[unsafe(link_section = ".multiboot2")]
#[unsafe(no_mangle)]
#[cfg(target_os = "none")]
pub static MULTIBOOT2_HEADER: MultibootHeader = MultibootHeader::new();

// Declare test_main as an external symbol when in test mode with kernel_test_mode feature
#[cfg(all(target_os = "none", test, feature = "kernel_test_mode"))]
extern "Rust" {
    fn test_main();
}

// 3. The Kernel Entry Point
#[unsafe(no_mangle)]
#[cfg(target_os = "none")]
pub extern "C" fn _start() -> ! {
    #[cfg(all(test, feature = "kernel_test_mode"))]
    {
        // Run tests instead of normal kernel boot
        unsafe {
            test_main();
        }
        // After tests complete, halt
        loop {}
    }

    #[cfg(not(all(test, feature = "kernel_test_mode")))]
    {
        let kernel = kernel_runtime::KernelRuntime::new();
        kernel.run();
    }
}

#[cfg(not(target_os = "none"))]
fn main() {}

/// Panic Handler
#[panic_handler]
#[cfg(target_os = "none")]
fn panic(info: &PanicInfo) -> ! {
    aethercore::kernel::panic_report(info, "panic");
}

// Test Runner
pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}
