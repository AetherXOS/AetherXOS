use core::sync::atomic::Ordering;
use crate::kernel_runtime::boot_info;

pub fn try_mount_initrd_once() {
    if super::super::INITRD_MOUNTED.load(Ordering::Relaxed) {
        return;
    }

    aethercore::kernel::debug_trace::record_optional(
        "main.loop",
        "initrd_mount_attempt",
        None,
        false,
    );

    #[cfg(feature = "vfs")]
    {
        let info_opt = boot_info::try_get();
        let Some(info) = info_opt else {
            return;
        };

        if let Some(module) = info.find_initrd() {
            if module.size == 0 {
                aethercore::klog_warn!("[INITRD] Module found but size=0, skipping mount");
                super::super::INITRD_MOUNTED.store(true, Ordering::Relaxed);
                return;
            }

            let virt_base = info.phys_to_virt(module.phys_base) as usize;
            let size = module.size as usize;

            // SAFETY: Limine guarantees the module memory is valid and
            // mapped for the lifetime of the kernel.
            let initrd_slice = unsafe { core::slice::from_raw_parts(virt_base as *const u8, size) };

            aethercore::klog_info!(
                "[INITRD] Mounting {} bytes from {:#x} ({})",
                size,
                module.phys_base,
                module.cmdline_str(),
            );

            let _ = initrd_slice;
            match aethercore::kernel::vfs_control::mount_ramfs(b"/") {
                Ok(_) => aethercore::klog_info!("[INITRD] Base ramfs mounted at /"),
                Err(e) => {
                    aethercore::klog_warn!(
                        "[INITRD] Mount fallback failed: {:?} — diskless mode",
                        e
                    )
                }
            }
        } else {
            aethercore::klog_info!("[INITRD] No initrd module provided — diskless mode");
            aethercore::klog_info!("[EARLY SERIAL] initrd not provided marker");
        }

        aethercore::klog_info!("[EARLY SERIAL] initrd mount complete before store");
        super::super::INITRD_MOUNTED.store(true, Ordering::Relaxed);
        aethercore::klog_info!("[EARLY SERIAL] initrd mount flag stored");
    }

    #[cfg(not(feature = "vfs"))]
    {
        super::super::INITRD_MOUNTED.store(true, Ordering::Relaxed);
    }
}

pub fn try_init_linux_compat_once() {
    aethercore::klog_info!("[EARLY SERIAL] try_init_linux_compat_once entry");
    if super::super::LINUX_COMPAT_INITED.load(Ordering::Relaxed) {
        aethercore::klog_info!("[EARLY SERIAL] linux_compat already initialized");
        return;
    }

    aethercore::kernel::debug_trace::record_optional(
        "main.loop",
        "linux_compat_init_attempt",
        None,
        false,
    );

    #[cfg(feature = "linux_compat")]
    {
        aethercore::klog_info!("[LINUX COMPAT] Initialising linux-compat layer");
        aethercore::modules::linux_compat::init();
        aethercore::klog_info!("[LINUX COMPAT] Ready");
        #[cfg(feature = "process_abstraction")]
        if super::super::LINKED_PROBE_ENABLED.load(Ordering::Relaxed) {
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "linux_compat_ready",
                None,
                false,
            );
            aethercore::klog_info!(
                "[LINKED PROBE] linux-compat ready; awaiting aether_init probe execution"
            );
        }
    }

    aethercore::klog_info!("[EARLY SERIAL] linux_compat init complete before store");
    super::super::LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
    aethercore::klog_info!("[EARLY SERIAL] linux_compat init flag stored");
}
