pub mod acl;
pub mod capabilities;
pub mod formal;
pub mod hardware;
pub mod mac;
pub mod null;
pub mod sel4;

use crate::interfaces::security::{ResourceKind, SecurityAction, SecurityContext, SecurityVerdict};
use crate::interfaces::task::TaskId;
use crate::interfaces::SecurityMonitor;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub use acl::AccessControlList;
pub use capabilities::ObjectCapability;
pub use formal::{formal_stats, register_proof_artifact, verify_security_invariants, FormalStats};
pub use hardware::{
    cpu_protection_status, detect_hardware_security, enforce_cpu_protections,
    hardware_security_stats, smap_disable_guard, CpuProtectionStatus, HardwareSecurityBackend,
    HardwareSecurityStats, SmapGuard,
};
pub use mac::{
    check_access as check_mac_access, set_resource_label as set_mac_resource_label,
    set_subject_clearance as set_mac_subject_clearance, MacLabel, MacStats,
};
pub use null::NullMonitor;
pub use sel4::SeL4Style;

// ─── Security Profile ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityProfile {
    /// No security enforcement — all access allowed.
    Null,
    /// Discretionary Access Control (DAC) via ACL.
    Acl,
    /// Capability-based access control.
    Capabilities,
    /// seL4-style deny-by-default with endpoint capabilities.
    Sel4,
    /// Zero Trust — combines MAC + Capabilities + ACL + Audit.
    ZeroTrust,
}

// ─── DMA Protection ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct DmaProtectionStatus {
    pub iommu_initialized: bool,
    pub hardware_mode: bool,
    pub backend: &'static str,
    pub protected_devices: usize,
    pub mapped_regions: usize,
}

// ─── Comprehensive Security Telemetry ───────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct SecurityTelemetry {
    pub profile: SecurityProfile,
    pub dma_active: bool,
    // ACL counters
    pub acl_grant_calls: u64,
    pub acl_revoke_calls: u64,
    pub acl_check_calls: u64,
    pub acl_check_hits: u64,
    pub acl_deny_calls: u64,
    // Capability counters
    pub cap_mint_calls: u64,
    pub cap_revoke_calls: u64,
    pub cap_access_calls: u64,
    pub cap_access_hits: u64,
    pub cap_access_denied: u64,
    pub cap_delegate_calls: u64,
    // MAC counters
    pub mac_check_calls: u64,
    pub mac_check_hits: u64,
    pub mac_check_denied: u64,
    // Formal verification
    pub formal_artifact_count: usize,
    pub formal_verify_passes: u64,
    // Hardware security
    pub hw_backend: hardware::HardwareSecurityBackend,
}

pub fn active_profile() -> SecurityProfile {
    if !crate::config::KernelConfig::security_enforcement_enabled() {
        return SecurityProfile::Null;
    }
    #[cfg(feature = "security_sel4")]
    {
        return SecurityProfile::Sel4;
    }
    #[cfg(all(not(feature = "security_sel4"), feature = "security_capabilities"))]
    {
        if crate::config::KernelConfig::capability_enforcement_enabled() {
            return SecurityProfile::Capabilities;
        }
    }
    #[cfg(all(
        not(feature = "security_sel4"),
        not(feature = "security_capabilities"),
        feature = "security_acl"
    ))]
    {
        return SecurityProfile::Acl;
    }
    SecurityProfile::Null
}

pub fn telemetry() -> SecurityTelemetry {
    let acl = acl::stats();
    let cap = capabilities::stats();
    let mac_stats = mac::stats();
    let formal = formal::formal_stats();
    SecurityTelemetry {
        profile: active_profile(),
        dma_active: is_dma_protection_active(),
        acl_grant_calls: acl.grant_calls,
        acl_revoke_calls: acl.revoke_calls,
        acl_check_calls: acl.check_calls,
        acl_check_hits: acl.check_hits,
        acl_deny_calls: acl.deny_calls,
        cap_mint_calls: cap.mint_calls,
        cap_revoke_calls: cap.revoke_calls,
        cap_access_calls: cap.access_calls,
        cap_access_hits: cap.access_hits,
        cap_access_denied: cap.access_denied,
        cap_delegate_calls: cap.delegate_calls,
        mac_check_calls: mac_stats.check_calls,
        mac_check_hits: mac_stats.check_hits,
        mac_check_denied: mac_stats.check_denied,
        formal_artifact_count: formal.artifact_count,
        formal_verify_passes: formal.verify_passes,
        hw_backend: detect_hardware_security(),
    }
}

pub fn dma_protection_status() -> DmaProtectionStatus {
    let stats = crate::hal::iommu::stats();
    DmaProtectionStatus {
        iommu_initialized: stats.initialized,
        hardware_mode: stats.hardware_mode,
        backend: stats.backend,
        protected_devices: stats.attached_devices,
        mapped_regions: stats.mapping_count,
    }
}

pub fn is_dma_protection_active() -> bool {
    let status = dma_protection_status();
    status.iommu_initialized && status.hardware_mode && status.protected_devices > 0
}

// ─── Resource ID Constants ──────────────────────────────────────────

pub const RESOURCE_VFS_MOUNT: u64 = 0x1001;
pub const RESOURCE_VFS_LIST: u64 = 0x1002;
pub const RESOURCE_VFS_PATH: u64 = 0x1003;
pub const RESOURCE_VFS_UNMOUNT: u64 = 0x1004;
pub const RESOURCE_VFS_STATS: u64 = 0x1005;
pub const RESOURCE_VFS_READ: u64 = 0x1006;
pub const RESOURCE_VFS_WRITE: u64 = 0x1007;
pub const RESOURCE_VFS_CREATE: u64 = 0x1008;
pub const RESOURCE_VFS_DELETE: u64 = 0x1009;
pub const RESOURCE_VFS_CHMOD: u64 = 0x100A;
pub const RESOURCE_VFS_CHOWN: u64 = 0x100B;
pub const RESOURCE_NETWORK_STATS: u64 = 0x2001;
pub const RESOURCE_NETWORK_CONTROL: u64 = 0x2002;
pub const RESOURCE_NETWORK_BIND: u64 = 0x2003;
pub const RESOURCE_NETWORK_CONNECT: u64 = 0x2004;
pub const RESOURCE_NETWORK_RAW: u64 = 0x2005;
pub const RESOURCE_IPC_FUTEX: u64 = 0x3001;
pub const RESOURCE_IPC_UPCALL: u64 = 0x3002;
pub const RESOURCE_IPC_CHANNEL: u64 = 0x3003;
pub const RESOURCE_IPC_SHARED_MEM: u64 = 0x3004;
pub const RESOURCE_POWER_STATS: u64 = 0x4001;
pub const RESOURCE_POWER_CONTROL: u64 = 0x4002;
pub const RESOURCE_POWER_REBOOT: u64 = 0x4003;
pub const RESOURCE_PROCESS_SPAWN: u64 = 0x5001;
pub const RESOURCE_PROCESS_KILL: u64 = 0x5002;
pub const RESOURCE_PROCESS_PTRACE: u64 = 0x5003;
pub const RESOURCE_MODULE_LOAD: u64 = 0x6001;
pub const RESOURCE_MODULE_UNLOAD: u64 = 0x6002;
pub const RESOURCE_SECURITY_POLICY: u64 = 0x7001;

// ─── Control-Plane Security ─────────────────────────────────────────

static CONTROL_BOOTSTRAPPED: AtomicBool = AtomicBool::new(false);
static ACL_CONTROL: AccessControlList = AccessControlList::new();
static CAP_CONTROL: ObjectCapability = ObjectCapability::new();

static CAP_VFS_MOUNT_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_VFS_LIST_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_VFS_PATH_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_VFS_UNMOUNT_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_VFS_STATS_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_NETWORK_STATS_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_NETWORK_CONTROL_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_POWER_STATS_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_POWER_CONTROL_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_IPC_FUTEX_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_IPC_UPCALL_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_PROCESS_SPAWN_TOKEN: AtomicU64 = AtomicU64::new(0);
static CAP_PROCESS_KILL_TOKEN: AtomicU64 = AtomicU64::new(0);

const CONTROL_TOKEN_SLOTS: &[(u64, &AtomicU64)] = &[
    (RESOURCE_VFS_MOUNT, &CAP_VFS_MOUNT_TOKEN),
    (RESOURCE_VFS_LIST, &CAP_VFS_LIST_TOKEN),
    (RESOURCE_VFS_PATH, &CAP_VFS_PATH_TOKEN),
    (RESOURCE_VFS_UNMOUNT, &CAP_VFS_UNMOUNT_TOKEN),
    (RESOURCE_VFS_STATS, &CAP_VFS_STATS_TOKEN),
    (RESOURCE_NETWORK_STATS, &CAP_NETWORK_STATS_TOKEN),
    (RESOURCE_NETWORK_CONTROL, &CAP_NETWORK_CONTROL_TOKEN),
    (RESOURCE_POWER_STATS, &CAP_POWER_STATS_TOKEN),
    (RESOURCE_POWER_CONTROL, &CAP_POWER_CONTROL_TOKEN),
    (RESOURCE_IPC_FUTEX, &CAP_IPC_FUTEX_TOKEN),
    (RESOURCE_IPC_UPCALL, &CAP_IPC_UPCALL_TOKEN),
    (RESOURCE_PROCESS_SPAWN, &CAP_PROCESS_SPAWN_TOKEN),
    (RESOURCE_PROCESS_KILL, &CAP_PROCESS_KILL_TOKEN),
];

#[inline(always)]
fn cap_token_slot(resource: u64) -> Option<&'static AtomicU64> {
    for (slot_resource, slot) in CONTROL_TOKEN_SLOTS {
        if *slot_resource == resource {
            return Some(*slot);
        }
    }
    None
}

#[inline(always)]
fn store_cap_token(resource: u64, token: u64) {
    if let Some(slot) = cap_token_slot(resource) {
        slot.store(token, Ordering::Relaxed);
    }
}

#[inline(always)]
fn cap_token_for_resource(resource: u64) -> u64 {
    cap_token_slot(resource).map(|slot| slot.load(Ordering::Relaxed)).unwrap_or(0)
}

fn bootstrap_control_security() {
    if CONTROL_BOOTSTRAPPED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let resources = [
        RESOURCE_VFS_MOUNT,
        RESOURCE_VFS_LIST,
        RESOURCE_VFS_PATH,
        RESOURCE_VFS_UNMOUNT,
        RESOURCE_VFS_STATS,
        RESOURCE_VFS_READ,
        RESOURCE_VFS_WRITE,
        RESOURCE_VFS_CREATE,
        RESOURCE_VFS_DELETE,
        RESOURCE_VFS_CHMOD,
        RESOURCE_VFS_CHOWN,
        RESOURCE_NETWORK_STATS,
        RESOURCE_NETWORK_CONTROL,
        RESOURCE_NETWORK_BIND,
        RESOURCE_NETWORK_CONNECT,
        RESOURCE_NETWORK_RAW,
        RESOURCE_POWER_STATS,
        RESOURCE_POWER_CONTROL,
        RESOURCE_POWER_REBOOT,
        RESOURCE_IPC_FUTEX,
        RESOURCE_IPC_UPCALL,
        RESOURCE_IPC_CHANNEL,
        RESOURCE_IPC_SHARED_MEM,
        RESOURCE_PROCESS_SPAWN,
        RESOURCE_PROCESS_KILL,
        RESOURCE_PROCESS_PTRACE,
        RESOURCE_MODULE_LOAD,
        RESOURCE_MODULE_UNLOAD,
        RESOURCE_SECURITY_POLICY,
    ];

    for resource in resources {
        ACL_CONTROL.grant_access(resource, TaskId(0));
        let token = CAP_CONTROL.mint_token(resource);
        store_cap_token(resource, token);
    }
}

/// Unified control-plane access check.
///
/// Checks access through the active security profile, then also checks MAC.
/// This is the single entry point for all kernel control-plane authorization.
pub fn check_control_plane_access(resource: u64) -> bool {
    bootstrap_control_security();
    let model_ok = match active_profile() {
        SecurityProfile::Null => true,
        SecurityProfile::Acl => ACL_CONTROL.check_access(resource),
        SecurityProfile::Capabilities => {
            let token = cap_token_for_resource(resource);
            token != 0 && CAP_CONTROL.check_access(token)
        }
        SecurityProfile::Sel4 => true, // seL4 has its own check path
        SecurityProfile::ZeroTrust => {
            // Zero Trust: require BOTH capability token AND ACL entry
            let token = cap_token_for_resource(resource);
            let cap_ok = token != 0 && CAP_CONTROL.check_access(token);
            let acl_ok = ACL_CONTROL.check_access(resource);
            cap_ok && acl_ok
        }
    };

    model_ok && mac::check_access(resource)
}

/// Full security check with context — preferred API for new code paths.
pub fn check_access_full(
    ctx: &SecurityContext,
    resource_id: u64,
    resource_kind: ResourceKind,
    action: SecurityAction,
) -> SecurityVerdict {
    bootstrap_control_security();

    // MAC check first (mandatory)
    let mac_verdict = mac::check_access_full(ctx, resource_id, action);
    if !mac_verdict.is_allowed() {
        return mac_verdict;
    }

    // Policy-specific check
    match active_profile() {
        SecurityProfile::Null => SecurityVerdict::Allow,
        SecurityProfile::Acl => {
            ACL_CONTROL.check_access_full(ctx, resource_id, resource_kind, action)
        }
        SecurityProfile::Capabilities => {
            CAP_CONTROL.check_access_full(ctx, resource_id, resource_kind, action)
        }
        SecurityProfile::Sel4 => {
            // seL4 uses its own static instance
            SecurityVerdict::Deny // Caller should use SeL4Style instance directly
        }
        SecurityProfile::ZeroTrust => {
            // Zero Trust: BOTH ACL and Capability must allow
            let acl = ACL_CONTROL.check_access_full(ctx, resource_id, resource_kind, action);
            if !acl.is_allowed() {
                return acl;
            }
            CAP_CONTROL.check_access_full(ctx, resource_id, resource_kind, action)
        }
    }
}

// ─── Active Security Selector ───────────────────────────────────────

pub mod selector {
    use super::*;

    #[cfg(feature = "security_null")]
    pub type ActiveSecurity = NullMonitor;

    #[cfg(feature = "security_acl")]
    pub type ActiveSecurity = AccessControlList;

    #[cfg(feature = "security_capabilities")]
    pub type ActiveSecurity = ObjectCapability;

    #[cfg(feature = "security_sel4")]
    pub type ActiveSecurity = SeL4Style;

    #[cfg(not(any(
        feature = "security_null",
        feature = "security_acl",
        feature = "security_capabilities",
        feature = "security_sel4"
    )))]
    pub type ActiveSecurity = NullMonitor;
}

#[cfg(test)]
mod tests {
    use super::{
        active_profile, cap_token_for_resource, check_control_plane_access, SecurityProfile,
        RESOURCE_PROCESS_KILL, RESOURCE_PROCESS_SPAWN,
    };

    #[test_case]
    fn process_control_resources_receive_bootstrap_tokens() {
        let _ = check_control_plane_access(RESOURCE_PROCESS_SPAWN);
        let _ = check_control_plane_access(RESOURCE_PROCESS_KILL);

        assert_ne!(cap_token_for_resource(RESOURCE_PROCESS_SPAWN), 0);
        assert_ne!(cap_token_for_resource(RESOURCE_PROCESS_KILL), 0);
    }

    #[test_case]
    fn runtime_security_toggle_can_reduce_profile_to_null() {
        crate::config::KernelConfig::reset_runtime_overrides();
        crate::config::KernelConfig::set_security_enforcement_enabled(Some(false));
        assert_eq!(active_profile(), SecurityProfile::Null);
        crate::config::KernelConfig::reset_runtime_overrides();
    }
}
