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

impl SecurityLevel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Unclassified,
            1 => Self::Confidential,
            2 => Self::Secret,
            3 => Self::TopSecret,
            4 => Self::KernelOnly,
            _ => Self::Unclassified,
        }
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

/// Per-task security context.
/// Carried by every task and inspected by the security monitor on each access check.
#[derive(Debug, Clone, Copy)]
pub struct SecurityContext {
    /// Task identity.
    pub task_id: TaskId,
    /// Owning process identity.
    pub process_id: ProcessId,
    /// Effective user ID (POSIX uid).
    pub euid: u32,
    /// Effective group ID (POSIX gid).
    pub egid: u32,
    /// Real user ID.
    pub ruid: u32,
    /// Real group ID.
    pub rgid: u32,
    /// Saved user ID (for setuid).
    pub suid: u32,
    /// Saved group ID (for setgid).
    pub sgid: u32,
    /// Base capability bitmask (first 64 capabilities).
    pub capabilities: u64,
    /// Ambient capability set — inherited across exec.
    pub ambient_caps: u64,
    /// MAC security level of the task.
    pub security_level: SecurityLevel,
    /// Namespace identifier (0 = root/default namespace).
    pub namespace_id: u32,
    /// Whether this context is in a privileged (admin/root) mode.
    pub privileged: bool,
    /// Whether security checks should be audited for this task.
    pub audit_enabled: bool,
}

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
            security_level: SecurityLevel::KernelOnly,
            namespace_id: 0,
            privileged: true,
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
            security_level: SecurityLevel::Unclassified,
            namespace_id: 0,
            privileged: false,
            audit_enabled: true,
        }
    }

    /// Check if this context holds a specific capability.
    #[inline(always)]
    pub fn has_capability(&self, cap: u64) -> bool {
        self.privileged || (self.capabilities & cap) == cap
    }

    /// Check if this context is running as root (euid 0).
    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.euid == 0
    }

    /// Check if this context can access the given security level.
    #[inline(always)]
    pub fn can_access_level(&self, required: SecurityLevel) -> bool {
        self.security_level >= required
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
