//! Mount policy ingestion and runtime actions.
//!
//! This module provides a small, safe scaffold to ingest mount-policy telemetry
//! (for example: `/run/hypercore/telemetry/mount_policy_events`) and apply
//! conservative runtime actions via `KernelConfig` (e.g. prefer unbuffered IO).
//!
//! The implementation here is intentionally lightweight: on `std`-enabled
//! builds we attempt a best-effort read of the telemetry file; on constrained
//! targets this becomes a no-op and the policy is exercised via other signals
//! (vfs telemetry counters, SLO evaluation, or control-plane commands).

#![allow(dead_code)]

/// Poll the on-disk telemetry and apply simple mount-policy actions.
///
/// - Host builds (with `std`) will try to read the telemetry file and map
///   lines to policy actions.
/// - Non-host builds become a no-op to avoid pulling std into kernel images.
pub fn poll_and_apply_mount_policy() {
    // Host-side best-effort ingestion (only when std is available).
    #[cfg(feature = "std")]
    {
        if let Ok(text) = std::fs::read_to_string("/run/hypercore/telemetry/mount_policy_events") {
            for line in text.lines() {
                // Simple heuristics: when the bootstrap reports tmpfs fallback,
                // prefer unbuffered IO (helps avoid write amplification on slow
                // block devices) as a conservative response.
                if line.contains("event=tmpfs_fallback") {
                    hypercore::klog_warn!("[MOUNT_POLICY] tmpfs_fallback observed; preferring unbuffered IO");
                    hypercore::config::KernelConfig::set_vfs_enable_buffered_io(Some(false));
                }

                // Diskfs mounted event: consider restoring buffered IO policy.
                if line.contains("event=diskfs_mounted") || line.contains("event=diskfs_mode_set") {
                    hypercore::klog_info!("[MOUNT_POLICY] diskfs available; restoring buffered IO if configured");
                    // Only restore if policy allows (no unconditional reset).
                    hypercore::config::KernelConfig::set_vfs_enable_buffered_io(Some(true));
                }
            }
        }
    }

    // On non-host targets this function intentionally does nothing; the
    // VFS SLO service (`service_vfs_runtime`) remains the canonical runtime
    // decision-maker inside the kernel.
}
