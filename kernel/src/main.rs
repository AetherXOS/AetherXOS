#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![feature(custom_test_frameworks)]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unused_must_use)]
#![allow(dead_code)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate aethercore;
extern crate alloc;

/// Multiboot2 header required for QEMU x86_64 boot.
/// Must be placed in the first 32 KB of the binary, 8-byte aligned,
/// and before the entry point.
#[repr(C, align(8))]
pub struct MultibootHeader {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    end_tag_type: u16,
    end_tag_flags: u16,
    end_tag_size: u32,
}

impl MultibootHeader {
    const fn new() -> Self {
        let magic = 0xE85250D6u32;
        let architecture = 0u32;
        let header_length = 12u32;
        let checksum = 0u32
            .wrapping_sub(magic)
            .wrapping_sub(architecture)
            .wrapping_sub(header_length);

        Self {
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

/// Test main symbol, provided by the test framework when `kernel_test_mode` is active.
#[cfg(all(target_os = "none", test, feature = "kernel_test_mode"))]
extern "Rust" {
    fn test_main();
}

/// Kernel entry point. Called by the bootloader.
#[unsafe(no_mangle)]
#[cfg(target_os = "none")]
pub extern "C" fn _start() -> ! {
    #[cfg(all(test, feature = "kernel_test_mode"))]
    {
        // SAFETY: test_main is defined by the test framework and is always safe to call
        // when kernel_test_mode is enabled. It runs all registered kernel tests.
        unsafe { test_main() }

        aethercore::klog_info!("KERNEL_TESTS: PASS");
        loop {}
    }

    #[cfg(not(all(test, feature = "kernel_test_mode")))]
    {
        let kernel = aethercore::kernel_runtime::KernelRuntime::new();
        kernel.run()
    }
}

/// Host-side stub: does nothing, exists only for `cargo test --target host` compatibility.
#[cfg(not(target_os = "none"))]
fn main() {}

/// Panic handler for the kernel.
#[panic_handler]
#[cfg(target_os = "none")]
fn panic(info: &core::panic::PanicInfo) -> ! {
    aethercore::kernel::panic_report(info, "panic");
}

/// Minimal test runner for `#[cfg(target_os = "none")]` tests.
pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}
