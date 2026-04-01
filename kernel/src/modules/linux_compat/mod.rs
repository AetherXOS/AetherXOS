#![allow(ambiguous_glob_reexports)]
#![allow(unused_imports)]

extern crate alloc;

pub(crate) use crate::kernel::syscalls::syscalls_consts::*;
pub(crate) use crate::kernel::syscalls::*;

pub mod error;
pub use self::error::*;
pub mod config;
pub use self::config::*;
pub mod config_surface;
pub use self::config_surface::*;
pub mod types;
pub use self::types::*;
#[macro_use]
pub mod helpers;
pub use self::helpers::*;
pub mod wrappers;
pub use self::wrappers::*;

pub mod process;
pub use self::process::*;
pub mod fs;
pub use self::fs::*;
pub mod net;
pub use self::net::*;
pub mod mem;
pub use self::mem::*;
pub mod sig;
pub use self::sig::*;
pub mod cred;
pub use self::cred::*;
pub mod time;
pub use self::time::*;
pub mod sync;
pub use self::sync::*;
pub mod sys;
pub use self::sys::*;
pub mod ipc;
pub use self::ipc::*;
pub mod errno_matrix;
pub use self::errno_matrix::*;
pub mod process_group_syscalls;
pub use self::process_group_syscalls::*;
pub mod standards;

#[cfg(feature = "ring_protection")]
pub mod sys_dispatcher;

// ── Module initialisation ─────────────────────────────────────────────────────

/// Initialise the linux-compat layer.
///
/// Must be called after the heap and VFS are ready.  Safe to call multiple
/// times — subsequent calls are no-ops.
pub fn init() {
    use core::sync::atomic::{AtomicBool, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::AcqRel) {
        return;
    }

    // Seed the PRNG from hardware RDRAND where available.
    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] linux compat seed begin\n");
        let seed: u64 = {
            let mut v: u64 = 0xDEAD_BEEF_CAFE_BABE;
            // SAFETY: rdrand is always present on our minimum target (Haswell+).
            unsafe {
                core::arch::asm!(
                    "2: rdrand {v}",
                    "jnc 2b",
                    v = out(reg) v,
                    options(nomem, nostack)
                );
            }
            v
        };
        crate::modules::linux_compat::config::set_prng_seed(seed);
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] linux compat seed returned\n");
    }

    // Initialise the standards dispatcher index.
    #[cfg(feature = "ring_protection")]
    {
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] linux compat dispatch index begin\n");
        crate::modules::linux_compat::sys_dispatcher::init_dispatch_index();
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] linux compat dispatch index returned\n");
    }

    #[cfg(feature = "vfs")]
    {
        // In some build targets (e.g. lib/test profiles), kernel_runtime is not linked.
        // Keep compat-surface init deterministic in those profiles.
        let linked_probe_boot = false;

        if linked_probe_boot {
            #[cfg(target_arch = "x86_64")]
            crate::hal::x86_64::serial::write_raw(
                "[EARLY SERIAL] linux compat surface deferred for linked probe\n",
            );
            crate::klog_info!(
                "[linux_compat] compat surface deferred during linked probe boot"
            );
        } else {
            #[cfg(target_arch = "x86_64")]
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] linux compat surface begin\n");
            match crate::modules::linux_compat::ensure_runtime_compat_surface_state() {
                Ok(Some(exported)) => {
                    crate::klog_info!(
                        "[linux_compat] compat surface mounted path={} exported_files={}",
                        crate::modules::linux_compat::DEFAULT_COMPAT_SURFACE_MOUNT_PATH,
                        exported
                    );
                }
                Ok(None) => {
                    crate::klog_info!("[linux_compat] compat surface remains hidden by policy");
                }
                Err(err) => {
                    crate::klog_warn!("[linux_compat] compat surface mount skipped: {}", err);
                }
            }
            #[cfg(target_arch = "x86_64")]
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] linux compat surface returned\n");
        }
    }

    #[cfg(target_arch = "x86_64")]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] linux compat init complete\n");
    crate::klog_info!("[linux_compat] init complete");
}
