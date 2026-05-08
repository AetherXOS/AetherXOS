// --- PLATFORM IMPLEMENTATIONS MODULE ---
// Concrete implementations of platform traits for supported architectures

#[cfg(target_arch = "x86_64")]
pub mod x86_64_platform;

#[cfg(target_arch = "aarch64")]
pub mod aarch64_platform;

#[cfg(target_arch = "x86_64")]
pub use x86_64_platform::X86_64_PLATFORM;

#[cfg(target_arch = "aarch64")]
pub use aarch64_platform::AARCH64_PLATFORM;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn platform() -> &'static dyn crate::interfaces::platform::Platform {
    &X86_64_PLATFORM
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn platform() -> &'static dyn crate::interfaces::platform::Platform {
    &AARCH64_PLATFORM
}

/// Get the current platform instance
pub fn get_platform() -> &'static dyn crate::interfaces::platform::Platform {
    platform()
}
