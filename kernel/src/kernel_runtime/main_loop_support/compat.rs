#[cfg(all(feature = "vfs", feature = "linux_compat"))]
pub(crate) fn refresh_linux_compat_surface() {
    use core::sync::atomic::Ordering;

    let sample = super::super::COMPAT_SURFACE_SAMPLE_COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    if let Ok(Some(_)) =
        hypercore::modules::linux_compat::maybe_refresh_runtime_compat_surface(sample)
    {
        let epoch = hypercore::modules::linux_compat::compat_surface_refresh_epoch();
        if sample % (1024 * 8) == 0 {
            hypercore::klog_info!("[LINUX COMPAT] compat surface refreshed epoch={}", epoch);
        }
    }
}
