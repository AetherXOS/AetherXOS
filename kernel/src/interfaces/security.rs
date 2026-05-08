// --- PILLAR 5: SECURITY (Production-Grade) ---

use crate::interfaces::task::{ProcessId, TaskId};

/// Security action types for fine-grained access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityAction {
    Read,
    Write,
    Execute,
    Create,
    Delete,
    Admin,
    Mount,
    Unmount,
    Signal,
    IpcSend,
    IpcRecv,
    NetBind,
    NetConnect,
    PtraceAttach,
    SetUid,
    SetGid,
    Chown,
    Chmod,
    ModuleLoad,
    ModuleUnload,
    Reboot,
    SetTime,
    RawIo,
    Capability(u16),
}

/// Resource kind classification for policy routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    File,
    Directory,
    Process,
    Thread,
    IpcChannel,
    NetworkSocket,
    Device,
    MountPoint,
    Memory,
    Interrupt,
    Syscall,
    Module,
    Capability,
    Namespace,
    SecurityPolicy,
    Custom(u64),
}

/// Security check verdict with audit support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityVerdict {
    /// Access is allowed.
    Allow,
    /// Access is denied.
    Deny,
    /// Access is allowed, but should be logged for audit.
    AuditAllow,
    /// Access is denied, and the denial should be logged for audit.
    AuditDeny,
}

impl SecurityVerdict {
    #[inline(always)]
    pub fn is_allowed(self) -> bool {
        matches!(self, SecurityVerdict::Allow | SecurityVerdict::AuditAllow)
    }

    #[inline(always)]
    pub fn should_audit(self) -> bool {
        matches!(
            self,
            SecurityVerdict::AuditAllow | SecurityVerdict::AuditDeny
        )
    }
}

/// MAC (Mandatory Access Control) security level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SecurityLevel {
    /// Unrestricted / public data.
    Unclassified = 0,
    /// Internal — limited distribution.
    Confidential = 1,
    /// Sensitive — need-to-know basis.
    Secret = 2,
    /// Highest classification.
    TopSecret = 3,
    /// Kernel-internal only — never exposed to userspace.
    KernelOnly = 4,
}

impl_enum_u8_default_conversions!(SecurityLevel {
    Unclassified,
    Confidential,
    Secret,
    TopSecret,
    KernelOnly,
}, default = Unclassified);

/// Security execution mode (compile-time selectable, runtime switch possible)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecurityMode {
    /// No security checks (zero overhead)
    Disabled = 0,
    /// Capability-based authorization only
    CapabilityOnly = 1,
    /// Full policy enforcement with audit trail
    PolicyEnforcement = 2,
}

impl SecurityMode {
    /// Check if capabilities should be checked
    #[inline(always)]
    pub fn has_capabilities(&self) -> bool {
        matches!(self, SecurityMode::CapabilityOnly | SecurityMode::PolicyEnforcement)
    }

    /// Check if policy/audit should be enforced
    #[inline(always)]
    pub fn has_policy(&self) -> bool {
        matches!(self, SecurityMode::PolicyEnforcement)
    }
}

/// Capability flags — first 64 capabilities as a bitmask for fast inline checks.
/// Capabilities beyond 64 use the extended capability set.
pub mod cap_flags {
    pub const CAP_CHOWN: u64 = 1 << 0;
    pub const CAP_FOWNER: u64 = 1 << 1;
    pub const CAP_KILL: u64 = 1 << 2;
    pub const CAP_SETUID: u64 = 1 << 3;
    pub const CAP_SETGID: u64 = 1 << 4;
    pub const CAP_NET_BIND: u64 = 1 << 5;
    pub const CAP_NET_RAW: u64 = 1 << 6;
    pub const CAP_NET_ADMIN: u64 = 1 << 7;
    pub const CAP_SYS_MODULE: u64 = 1 << 8;
    pub const CAP_SYS_RAWIO: u64 = 1 << 9;
    pub const CAP_SYS_ADMIN: u64 = 1 << 10;
    pub const CAP_SYS_BOOT: u64 = 1 << 11;
    pub const CAP_SYS_TIME: u64 = 1 << 12;
    pub const CAP_IPC_LOCK: u64 = 1 << 13;
    pub const CAP_IPC_OWNER: u64 = 1 << 14;
    pub const CAP_SYS_PTRACE: u64 = 1 << 15;
    pub const CAP_MKNOD: u64 = 1 << 16;
    pub const CAP_LEASE: u64 = 1 << 17;
    pub const CAP_AUDIT_WRITE: u64 = 1 << 18;
    pub const CAP_AUDIT_CONTROL: u64 = 1 << 19;
    pub const CAP_MOUNT: u64 = 1 << 20;
    pub const CAP_UNMOUNT: u64 = 1 << 21;
    pub const CAP_DAC_OVERRIDE: u64 = 1 << 22;
    pub const CAP_DAC_READ_SEARCH: u64 = 1 << 23;
    pub const CAP_SYS_NICE: u64 = 1 << 24;
    pub const CAP_SYS_RESOURCE: u64 = 1 << 25;
    pub const CAP_WAKE_ALARM: u64 = 1 << 26;
    pub const CAP_BLOCK_SUSPEND: u64 = 1 << 27;
    pub const CAP_SYSLOG: u64 = 1 << 28;
    pub const CAP_MAC_ADMIN: u64 = 1 << 29;
    pub const CAP_MAC_OVERRIDE: u64 = 1 << 30;
    pub const CAP_PERFMON: u64 = 1 << 31;

    /// Root has all base capabilities.
    pub const CAP_ALL: u64 = u64::MAX;
    /// No capabilities — fully unprivileged.
    pub const CAP_NONE: u64 = 0;
}

/// Per-task security context (optional capabilities via feature flag).
/// Carried by every task and inspected by the security monitor on each access check.
#[derive(Debug, Clone, Copy)]
pub struct SecurityContext {
    // Always present
    pub euid: u32,              // Effective user ID (POSIX uid)
    pub egid: u32,              // Effective group ID (POSIX gid)
    
    // Optional via feature flag: security_capabilities
    #[cfg(feature = "capability_system")]
    pub task_id: TaskId,        // Task identity
    #[cfg(feature = "capability_system")]
    pub process_id: ProcessId,  // Owning process identity
    #[cfg(feature = "capability_system")]
    pub ruid: u32,              // Real user ID
    #[cfg(feature = "capability_system")]
    pub rgid: u32,              // Real group ID
    #[cfg(feature = "capability_system")]
    pub suid: u32,              // Saved user ID (for setuid)
    #[cfg(feature = "capability_system")]
    pub sgid: u32,              // Saved group ID (for setgid)
    #[cfg(feature = "capability_system")]
    pub capabilities: u64,      // Base capability bitmask (first 64 capabilities)
    #[cfg(feature = "capability_system")]
    pub ambient_caps: u64,      // Ambient capability set — inherited across exec
    
    // Optional via feature flag: policy_enforcement
    #[cfg(feature = "policy_enforcement")]
    pub security_level: SecurityLevel,  // MAC security level
    #[cfg(feature = "policy_enforcement")]
    pub namespace_id: u32,              // Namespace identifier (0 = root)
    #[cfg(feature = "policy_enforcement")]
    pub privileged: bool,               // Privileged mode flag
    #[cfg(feature = "policy_enforcement")]
    pub audit_enabled: bool,            // Audit this task's security events
}

// Impl for capability_system feature
#[cfg(feature = "capability_system")]
impl SecurityContext {
    /// Create a kernel-level (fully privileged) context.
    pub const fn kernel() -> Self {
        Self {
            task_id: TaskId(0),
            process_id: ProcessId(0),
            euid: 0,
            egid: 0,
            ruid: 0,
            rgid: 0,
            suid: 0,
            sgid: 0,
            capabilities: cap_flags::CAP_ALL,
            ambient_caps: cap_flags::CAP_ALL,
            #[cfg(feature = "policy_enforcement")]
            security_level: SecurityLevel::KernelOnly,
            #[cfg(feature = "policy_enforcement")]
            namespace_id: 0,
            #[cfg(feature = "policy_enforcement")]
            privileged: true,
            #[cfg(feature = "policy_enforcement")]
            audit_enabled: false,
        }
    }

    /// Create an unprivileged user context.
    pub const fn user(task_id: TaskId, process_id: ProcessId, uid: u32, gid: u32) -> Self {
        Self {
            task_id,
            process_id,
            euid: uid,
            egid: gid,
            ruid: uid,
            rgid: gid,
            suid: uid,
            sgid: gid,
            capabilities: cap_flags::CAP_NONE,
            ambient_caps: cap_flags::CAP_NONE,
            #[cfg(feature = "policy_enforcement")]
            security_level: SecurityLevel::Unclassified,
            #[cfg(feature = "policy_enforcement")]
            namespace_id: 0,
            #[cfg(feature = "policy_enforcement")]
            privileged: false,
            #[cfg(feature = "policy_enforcement")]
            audit_enabled: true,
        }
    }

    /// Check if this context holds a specific capability.
    #[inline(always)]
    pub fn has_capability(&self, cap: u64) -> bool {
        #[cfg(feature = "policy_enforcement")]
        {
            self.privileged || (self.capabilities & cap) == cap
        }
        #[cfg(all(feature = "capability_system", not(feature = "policy_enforcement")))]
        {
            (self.capabilities & cap) == cap
        }
    }

    /// Check if this context is running as root (euid 0).
    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.euid == 0
    }
}

// Impl for policy_enforcement feature only
#[cfg(feature = "policy_enforcement")]
impl SecurityContext {
    /// Check if this context can access the given security level.
    #[inline(always)]
    pub fn can_access_level(&self, required: SecurityLevel) -> bool {
        self.security_level >= required
    }
}

// Impl for contexts without capability_system (minimal)
#[cfg(not(feature = "capability_system"))]
impl SecurityContext {
    /// Create a minimal context for lightweight systems.
    pub const fn minimal(uid: u32, gid: u32) -> Self {
        Self {
            euid: uid,
            egid: gid,
        }
    }

    /// Check if this context is running as root (euid 0).
    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.euid == 0
    }
}

/// Resource limits for processes and tasks.
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// Maximum number of open file descriptors.
    pub max_open_files: u32,
    /// Maximum virtual memory size in bytes (0 = unlimited).
    pub max_vm_bytes: u64,
    /// Maximum number of threads per process.
    pub max_threads: u32,
    /// Maximum CPU time in nanoseconds (0 = unlimited).
    pub max_cpu_ns: u64,
    /// Maximum stack size in bytes.
    pub max_stack_bytes: u64,
    /// Maximum heap size in bytes (0 = unlimited).
    pub max_heap_bytes: u64,
    /// Maximum number of IPC channels.
    pub max_ipc_channels: u32,
    /// Maximum number of pending signals.
    pub max_pending_signals: u32,
    /// Maximum number of child processes.
    pub max_children: u32,
    /// Maximum locked memory in bytes.
    pub max_locked_memory: u64,
    /// Maximum number of processes this user can own (RLIMIT_NPROC). 0 = unlimited.
    pub max_processes: usize,
    /// Real user ID (for fork inheritance).
    pub uid: u32,
    /// Real group ID (for fork inheritance).
    pub gid: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::unlimited()
    }
}

impl ResourceLimits {
    /// Unlimited — no restrictions.
    pub const fn unlimited() -> Self {
        Self {
            max_open_files: u32::MAX,
            max_vm_bytes: 0,
            max_threads: u32::MAX,
            max_cpu_ns: 0,
            max_stack_bytes: 0,
            max_heap_bytes: 0,
            max_ipc_channels: u32::MAX,
            max_pending_signals: 256,
            max_children: u32::MAX,
            max_locked_memory: 0,
            max_processes: 0,
            uid: 0,
            gid: 0,
        }
    }

    /// Default user limits.
    pub const fn default_user() -> Self {
        Self {
            max_open_files: 1024,
            max_vm_bytes: 256 * 1024 * 1024, // 256 MiB
            max_threads: 64,
            max_cpu_ns: 0,                     // unlimited
            max_stack_bytes: 2 * 1024 * 1024,  // 2 MiB
            max_heap_bytes: 128 * 1024 * 1024, // 128 MiB
            max_ipc_channels: 128,
            max_pending_signals: 64,
            max_children: 128,
            max_locked_memory: 16 * 1024 * 1024, // 16 MiB
            max_processes: 256,
            uid: u32::MAX, // set by runtime
            gid: u32::MAX,
        }
    }
}

/// The production-grade Security Monitor trait.
///
/// Every security policy backend (Null, ACL, Capability, RBAC, SeL4, Zero Trust)
/// must implement this trait. The kernel calls these methods at each enforcement point.
pub trait SecurityMonitor {
    /// Simple access check (legacy / fast path).
    fn check_access(&self, resource_id: u64) -> bool;

    /// Full access check with context, action, and resource classification.
    fn check_access_full(
        &self,
        _ctx: &SecurityContext,
        resource_id: u64,
        _resource_kind: ResourceKind,
        _action: SecurityAction,
    ) -> SecurityVerdict {
        // Default implementation delegates to simple check_access.
        // Backends override this for policy-specific logic.
        if self.check_access(resource_id) {
            SecurityVerdict::Allow
        } else {
            SecurityVerdict::Deny
        }
    }

    /// Grant access to a resource for a specific security context.
    fn grant(
        &self,
        _ctx: &SecurityContext,
        _resource_id: u64,
        _resource_kind: ResourceKind,
        _action: SecurityAction,
    ) -> bool {
        false // Default: grants not supported
    }

    /// Revoke access to a resource for a specific security context.
    fn revoke(
        &self,
        _ctx: &SecurityContext,
        _resource_id: u64,
        _resource_kind: ResourceKind,
        _action: SecurityAction,
    ) -> bool {
        false // Default: revokes not supported
    }

    /// Check if a task holds a specific capability.
    fn has_capability(&self, ctx: &SecurityContext, cap: u64) -> bool {
        ctx.has_capability(cap)
    }

    /// Validate resource limits before allowing a resource allocation.
    fn check_resource_limit(
        &self,
        _ctx: &SecurityContext,
        limits: &ResourceLimits,
        resource_kind: ResourceKind,
        current_count: u64,
    ) -> bool {
        match resource_kind {
            ResourceKind::File => current_count < limits.max_open_files as u64,
            ResourceKind::Thread => current_count < limits.max_threads as u64,
            ResourceKind::IpcChannel => current_count < limits.max_ipc_channels as u64,
            ResourceKind::Process => current_count < limits.max_children as u64,
            _ => true,
        }
    }

    /// Return the name of the security policy backend.
    fn policy_name(&self) -> &'static str {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_mode_variants() {
        assert_eq!(SecurityMode::Disabled as u8, 0);
        assert_eq!(SecurityMode::CapabilityOnly as u8, 1);
        assert_eq!(SecurityMode::PolicyEnforcement as u8, 2);
    }

    #[test]
    fn test_security_mode_checks() {
        assert!(!SecurityMode::Disabled.has_capabilities());
        assert!(!SecurityMode::Disabled.has_policy());

        assert!(SecurityMode::CapabilityOnly.has_capabilities());
        assert!(!SecurityMode::CapabilityOnly.has_policy());

        assert!(SecurityMode::PolicyEnforcement.has_capabilities());
        assert!(SecurityMode::PolicyEnforcement.has_policy());
    }

    #[test]
    fn test_capability_flags() {
        // Verify individual capability bits
        assert_eq!(cap_flags::CAP_CHOWN, 1 << 0);
        assert_eq!(cap_flags::CAP_SETUID, 1 << 3);
        assert_eq!(cap_flags::CAP_SYS_ADMIN, 1 << 10);

        // Verify bitwise operations
        let caps = cap_flags::CAP_CHOWN | cap_flags::CAP_SETUID;
        assert_eq!(caps & cap_flags::CAP_CHOWN, cap_flags::CAP_CHOWN);
        assert_eq!(caps & cap_flags::CAP_SETUID, cap_flags::CAP_SETUID);
        assert_eq!(caps & cap_flags::CAP_SYS_ADMIN, 0);
    }

    #[test]
    fn test_security_verdict_is_allowed() {
        assert!(SecurityVerdict::Allow.is_allowed());
        assert!(SecurityVerdict::AuditAllow.is_allowed());
        assert!(!SecurityVerdict::Deny.is_allowed());
        assert!(!SecurityVerdict::AuditDeny.is_allowed());
    }

    #[test]
    fn test_security_verdict_should_audit() {
        assert!(!SecurityVerdict::Allow.should_audit());
        assert!(SecurityVerdict::AuditAllow.should_audit());
        assert!(!SecurityVerdict::Deny.should_audit());
        assert!(SecurityVerdict::AuditDeny.should_audit());
    }

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Unclassified < SecurityLevel::Confidential);
        assert!(SecurityLevel::Confidential < SecurityLevel::Secret);
        assert!(SecurityLevel::Secret < SecurityLevel::TopSecret);
        assert!(SecurityLevel::TopSecret < SecurityLevel::KernelOnly);
    }

    #[cfg(feature = "capability_system")]
    #[test]
    fn test_security_context_is_root() {
        let root = SecurityContext::user(TaskId(1), ProcessId(1), 0, 0);
        assert!(root.is_root());

        let user = SecurityContext::user(TaskId(2), ProcessId(2), 1000, 1000);
        assert!(!user.is_root());
    }

    #[cfg(feature = "capability_system")]
    #[test]
    fn test_security_context_kernel() {
        let ctx = SecurityContext::kernel();
        assert_eq!(ctx.euid, 0);
        assert_eq!(ctx.egid, 0);
        assert_eq!(ctx.capabilities, cap_flags::CAP_ALL);
        assert_eq!(ctx.ambient_caps, cap_flags::CAP_ALL);
    }

    #[cfg(feature = "capability_system")]
    #[test]
    fn test_security_context_user() {
        let ctx = SecurityContext::user(TaskId(42), ProcessId(100), 1000, 1000);
        assert_eq!(ctx.task_id, TaskId(42));
        assert_eq!(ctx.process_id, ProcessId(100));
        assert_eq!(ctx.euid, 1000);
        assert_eq!(ctx.egid, 1000);
        assert_eq!(ctx.capabilities, cap_flags::CAP_NONE);
        assert_eq!(ctx.ambient_caps, cap_flags::CAP_NONE);
    }

    #[cfg(all(feature = "capability_system", feature = "policy_enforcement"))]
    #[test]
    fn test_security_context_privileged_bypass() {
        let mut root = SecurityContext::kernel();
        // Kernel context is privileged
        assert!(root.has_capability(cap_flags::CAP_CHOWN));
        assert!(root.has_capability(cap_flags::CAP_SYS_ADMIN));
    }

    #[cfg(feature = "capability_system")]
    #[test]
    fn test_security_context_capability_check() {
        let mut ctx = SecurityContext::user(TaskId(1), ProcessId(1), 1000, 1000);
        #[cfg(feature = "policy_enforcement")]
        {
            ctx.privileged = false;
        }
        // Unprivileged user should fail capability checks
        assert!(!ctx.has_capability(cap_flags::CAP_CHOWN));
        assert!(!ctx.has_capability(cap_flags::CAP_SYS_ADMIN));
    }

    #[cfg(feature = "policy_enforcement")]
    #[test]
    fn test_security_context_can_access_level() {
        let root = SecurityContext::kernel();
        assert!(root.can_access_level(SecurityLevel::Unclassified));
        assert!(root.can_access_level(SecurityLevel::KernelOnly));

        let mut user = SecurityContext::user(TaskId(1), ProcessId(1), 1000, 1000);
        user.security_level = SecurityLevel::Confidential;
        assert!(user.can_access_level(SecurityLevel::Unclassified));
        assert!(user.can_access_level(SecurityLevel::Confidential));
        assert!(!user.can_access_level(SecurityLevel::Secret));
    }

    #[cfg(not(feature = "capability_system"))]
    #[test]
    fn test_security_context_minimal() {
        let ctx = SecurityContext::minimal(1000, 1000);
        assert_eq!(ctx.euid, 1000);
        assert_eq!(ctx.egid, 1000);
    }

    #[test]
    fn test_resource_limits_defaults() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_open_files, 1024);
        assert_eq!(limits.max_vm_bytes, 256 * 1024 * 1024);
        assert_eq!(limits.max_threads, 64);
        assert_eq!(limits.max_stack_bytes, 2 * 1024 * 1024);
    }

    #[test]
    fn test_resource_limits_stack_heap() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_heap_bytes, 128 * 1024 * 1024);
        assert_eq!(limits.max_ipc_channels, 256);
        assert_eq!(limits.max_children, 128);
    }
}
