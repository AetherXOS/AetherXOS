pub mod cgroup_ns;
pub mod ipc_ns;
pub mod mount_ns;
pub mod net_ns;
/// Process Namespace Isolation Framework.
///
/// Implements Linux-compatible namespace types for resource isolation:
/// - **PID namespace**: Isolated PID number spaces.
/// - **Mount namespace**: Per-process mount table views.
/// - **Network namespace**: Isolated network stacks.
/// - **UTS namespace**: Isolated hostname / domainname.
/// - **IPC namespace**: Isolated System V IPC / POSIX mq.
/// - **User namespace**: UID/GID mapping.
/// - **Cgroup namespace**: Cgroup root visibility.
///
/// ## Architecture
///
/// Each namespace type is behind a feature flag gated at compile time
/// (`namespace_pid`, `namespace_net`, etc.) and can be additionally
/// toggled at runtime through `NamespaceConfig`.
///
/// Namespaces form a hierarchy: child namespaces inherit from parents
/// unless explicitly detached via `unshare()` or `clone(CLONE_NEW*)`.
pub mod pid_ns;
pub mod user_ns;
pub mod uts_ns;

use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use lazy_static::lazy_static;

// Re-exports
pub use cgroup_ns::CgroupNamespace;
pub use ipc_ns::IpcNamespace;
pub use mount_ns::MountNamespace;
pub use net_ns::NetNamespace;
pub use pid_ns::PidNamespace;
pub use user_ns::UserNamespace;
pub use uts_ns::UtsNamespace;

// ─── Telemetry ───────────────────────────────────────────────────────

static NS_CREATES: AtomicU64 = AtomicU64::new(0);
static NS_DESTROYS: AtomicU64 = AtomicU64::new(0);
static NS_UNSHARE_CALLS: AtomicU64 = AtomicU64::new(0);
static NS_SETNS_CALLS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct NamespaceStats {
    pub creates: u64,
    pub destroys: u64,
    pub unshare_calls: u64,
    pub setns_calls: u64,
}

pub fn namespace_stats() -> NamespaceStats {
    NamespaceStats {
        creates: NS_CREATES.load(Ordering::Relaxed),
        destroys: NS_DESTROYS.load(Ordering::Relaxed),
        unshare_calls: NS_UNSHARE_CALLS.load(Ordering::Relaxed),
        setns_calls: NS_SETNS_CALLS.load(Ordering::Relaxed),
    }
}

// ─── Namespace Type ──────────────────────────────────────────────────

bitflags::bitflags! {
    /// Clone flags for namespace creation (matches Linux CLONE_NEW* bits).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CloneFlags: u32 {
        const CLONE_NEWPID    = 0x2000_0000;
        const CLONE_NEWNET    = 0x4000_0000;
        const CLONE_NEWNS     = 0x0002_0000; // mount
        const CLONE_NEWIPC    = 0x0800_0000;
        const CLONE_NEWUTS    = 0x0400_0000;
        const CLONE_NEWUSER   = 0x1000_0000;
        const CLONE_NEWCGROUP = 0x0200_0000;
    }
}

/// Identifies a namespace type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NsType {
    Pid,
    Net,
    Mount,
    Ipc,
    Uts,
    User,
    Cgroup,
}

/// Unique namespace identifier.
pub type NsId = u64;

static NEXT_NS_ID: AtomicU64 = AtomicU64::new(1);

static NEXT_NSSET_ID: AtomicU64 = AtomicU64::new(1);
/// Namespace file-descriptor counter. Starts at 1000 to sit above typical process FDs.
static NEXT_NSFD: AtomicI32 = AtomicI32::new(1000);

lazy_static! {
    static ref NSSET_TABLE: IrqSafeMutex<BTreeMap<u32, NsSet>> = {
        let mut map = BTreeMap::new();
        map.insert(0, NsSet::init_root());
        IrqSafeMutex::new(map)
    };
    /// Maps namespace file-descriptor numbers to their `ns_set_id`.
    static ref NSFD_TABLE: IrqSafeMutex<BTreeMap<i32, u32>> =
        IrqSafeMutex::new(BTreeMap::new());
}

/// Allocate a globally unique namespace ID.
pub fn alloc_ns_id() -> NsId {
    NEXT_NS_ID.fetch_add(1, Ordering::Relaxed)
}

#[inline(always)]
fn namespace_flag_mask() -> u32 {
    CloneFlags::CLONE_NEWPID.bits()
        | CloneFlags::CLONE_NEWNET.bits()
        | CloneFlags::CLONE_NEWNS.bits()
        | CloneFlags::CLONE_NEWIPC.bits()
        | CloneFlags::CLONE_NEWUTS.bits()
        | CloneFlags::CLONE_NEWUSER.bits()
        | CloneFlags::CLONE_NEWCGROUP.bits()
}

#[inline(always)]
fn nstype_to_ns_type(nstype: u32) -> Option<NsType> {
    match nstype {
        x if x == CloneFlags::CLONE_NEWPID.bits() => Some(NsType::Pid),
        x if x == CloneFlags::CLONE_NEWNET.bits() => Some(NsType::Net),
        x if x == CloneFlags::CLONE_NEWNS.bits() => Some(NsType::Mount),
        x if x == CloneFlags::CLONE_NEWIPC.bits() => Some(NsType::Ipc),
        x if x == CloneFlags::CLONE_NEWUTS.bits() => Some(NsType::Uts),
        x if x == CloneFlags::CLONE_NEWUSER.bits() => Some(NsType::User),
        x if x == CloneFlags::CLONE_NEWCGROUP.bits() => Some(NsType::Cgroup),
        _ => None,
    }
}

/// Return a cloned namespace set for a process namespace-id.
pub fn namespace_set_by_id(id: u32) -> Option<NsSet> {
    NSSET_TABLE.lock().get(&id).cloned()
}

/// Ensure that `id` exists in the namespace table (falls back to root clone).
pub fn ensure_namespace_set(id: u32) {
    let mut table = NSSET_TABLE.lock();
    if table.contains_key(&id) {
        return;
    }
    let root = table.get(&0).cloned().unwrap_or_else(NsSet::init_root);
    table.insert(id, root);
}

/// Apply `unshare(2)` semantics to a process namespace handle and return
/// the newly allocated namespace-id.
pub fn unshare_process_namespaces(current_id: u32, flags_raw: u32) -> Result<u32, &'static str> {
    let supported = namespace_flag_mask();
    if (flags_raw & !supported) != 0 {
        return Err("EINVAL");
    }

    if flags_raw == 0 {
        return Ok(current_id);
    }

    let flags = CloneFlags::from_bits(flags_raw).ok_or("EINVAL")?;

    let mut table = NSSET_TABLE.lock();
    let base = table
        .get(&current_id)
        .cloned()
        .or_else(|| table.get(&0).cloned())
        .unwrap_or_else(NsSet::init_root);

    let child = base.unshare(flags);
    let next = NEXT_NSSET_ID.fetch_add(1, Ordering::Relaxed);
    let id = u32::try_from(next).map_err(|_| "EOVERFLOW")?;
    table.insert(id, child);
    Ok(id)
}

// ─── Namespace Set ───────────────────────────────────────────────────

/// The full set of namespaces associated with a process.
/// Each field is reference-counted so namespaces can be shared
/// across processes that haven't called `unshare()`.
#[derive(Clone)]
pub struct NsSet {
    pub pid_ns: Arc<PidNamespace>,
    pub net_ns: Arc<NetNamespace>,
    pub mount_ns: Arc<MountNamespace>,
    pub ipc_ns: Arc<IpcNamespace>,
    pub uts_ns: Arc<UtsNamespace>,
    pub user_ns: Arc<UserNamespace>,
    pub cgroup_ns: Arc<CgroupNamespace>,
}

impl NsSet {
    /// Create the initial (root) namespace set.
    pub fn init_root() -> Self {
        NS_CREATES.fetch_add(7, Ordering::Relaxed);
        Self {
            pid_ns: Arc::new(PidNamespace::root()),
            net_ns: Arc::new(NetNamespace::root()),
            mount_ns: Arc::new(MountNamespace::root()),
            ipc_ns: Arc::new(IpcNamespace::root()),
            uts_ns: Arc::new(UtsNamespace::root()),
            user_ns: Arc::new(UserNamespace::root()),
            cgroup_ns: Arc::new(CgroupNamespace::root()),
        }
    }

    /// Unshare the specified namespaces, creating fresh copies.
    pub fn unshare(&self, flags: CloneFlags) -> Self {
        NS_UNSHARE_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut new = self.clone();
        if flags.contains(CloneFlags::CLONE_NEWPID) {
            NS_CREATES.fetch_add(1, Ordering::Relaxed);
            new.pid_ns = Arc::new(PidNamespace::child_of(&self.pid_ns));
        }
        if flags.contains(CloneFlags::CLONE_NEWNET) {
            NS_CREATES.fetch_add(1, Ordering::Relaxed);
            new.net_ns = Arc::new(NetNamespace::new());
        }
        if flags.contains(CloneFlags::CLONE_NEWNS) {
            NS_CREATES.fetch_add(1, Ordering::Relaxed);
            new.mount_ns = Arc::new(MountNamespace::clone_from(&self.mount_ns));
        }
        if flags.contains(CloneFlags::CLONE_NEWIPC) {
            NS_CREATES.fetch_add(1, Ordering::Relaxed);
            new.ipc_ns = Arc::new(IpcNamespace::new());
        }
        if flags.contains(CloneFlags::CLONE_NEWUTS) {
            NS_CREATES.fetch_add(1, Ordering::Relaxed);
            new.uts_ns = Arc::new(UtsNamespace::clone_from(&self.uts_ns));
        }
        if flags.contains(CloneFlags::CLONE_NEWUSER) {
            NS_CREATES.fetch_add(1, Ordering::Relaxed);
            new.user_ns = Arc::new(UserNamespace::new());
        }
        if flags.contains(CloneFlags::CLONE_NEWCGROUP) {
            NS_CREATES.fetch_add(1, Ordering::Relaxed);
            new.cgroup_ns = Arc::new(CgroupNamespace::new());
        }
        new
    }

    /// Enter (setns) a specific namespace from another set.
    pub fn setns(&mut self, ns_type: NsType, source: &NsSet) {
        NS_SETNS_CALLS.fetch_add(1, Ordering::Relaxed);
        match ns_type {
            NsType::Pid => self.pid_ns = Arc::clone(&source.pid_ns),
            NsType::Net => self.net_ns = Arc::clone(&source.net_ns),
            NsType::Mount => self.mount_ns = Arc::clone(&source.mount_ns),
            NsType::Ipc => self.ipc_ns = Arc::clone(&source.ipc_ns),
            NsType::Uts => self.uts_ns = Arc::clone(&source.uts_ns),
            NsType::User => self.user_ns = Arc::clone(&source.user_ns),
            NsType::Cgroup => self.cgroup_ns = Arc::clone(&source.cgroup_ns),
        }
    }

    /// List namespace IDs for introspection / procfs.
    pub fn ns_ids(&self) -> Vec<(NsType, NsId)> {
        alloc::vec![
            (NsType::Pid, self.pid_ns.id),
            (NsType::Net, self.net_ns.id),
            (NsType::Mount, self.mount_ns.id),
            (NsType::Ipc, self.ipc_ns.id),
            (NsType::Uts, self.uts_ns.id),
            (NsType::User, self.user_ns.id),
            (NsType::Cgroup, self.cgroup_ns.id),
        ]
    }
}

// ─── Namespace File Descriptors ──────────────────────────────────────

/// Open a namespace file descriptor for `ns_set_id`.
///
/// The returned fd can be passed to `setns(2)` to re-enter the namespace.
pub fn nsfd_open(ns_set_id: u32) -> i32 {
    let fd = NEXT_NSFD.fetch_add(1, Ordering::Relaxed);
    NSFD_TABLE.lock().insert(fd, ns_set_id);
    fd
}

/// Release a namespace file descriptor.
pub fn nsfd_close(fd: i32) {
    NSFD_TABLE.lock().remove(&fd);
}

/// Implement `setns(2)` semantics: resolve `nsfd` to an `NsSet` and assign it to the
/// calling process, returning a new namespace-id.
///
/// If `nstype` is 0, the caller adopts the full target set. Otherwise only the selected
/// namespace type is reassociated.
pub fn setns_process_namespaces(
    current_id: u32,
    nsfd: i32,
    nstype: u32,
) -> Result<u32, &'static str> {
    // Step 1: resolve fd → ns_set_id
    let target_set_id = {
        let table = NSFD_TABLE.lock();
        *table.get(&nsfd).ok_or("EBADF")?
    };

    // Step 2: clone the target NsSet
    let target_set = {
        let table = NSSET_TABLE.lock();
        table.get(&target_set_id).cloned().ok_or("EINVAL")?
    };

    // Step 3: apply full join or selective join by namespace type.
    let new_set = if nstype == 0 {
        target_set
    } else {
        let ns_type = nstype_to_ns_type(nstype).ok_or("EINVAL")?;
        let table = NSSET_TABLE.lock();
        let base = table
            .get(&current_id)
            .cloned()
            .or_else(|| table.get(&0).cloned())
            .ok_or("EINVAL")?;
        drop(table);
        let mut joined = base;
        joined.setns(ns_type, &target_set);
        joined
    };

    // Step 4: allocate a new id for the caller's updated namespace state and store it.
    let next = NEXT_NSSET_ID.fetch_add(1, Ordering::Relaxed);
    let new_id = u32::try_from(next).map_err(|_| "EOVERFLOW")?;
    NSSET_TABLE.lock().insert(new_id, new_set);

    NS_SETNS_CALLS.fetch_add(1, Ordering::Relaxed);
    Ok(new_id)
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_unshare_zero_flags_is_noop() {
        ensure_namespace_set(0);
        let id = unshare_process_namespaces(0, 0).expect("zero flags must succeed");
        assert_eq!(id, 0, "zero flags must return the same namespace id");
    }

    #[test_case]
    fn test_unshare_pid_allocates_unique_ids() {
        ensure_namespace_set(0);
        let id1 = unshare_process_namespaces(0, CloneFlags::CLONE_NEWPID.bits())
            .expect("CLONE_NEWPID unshare must succeed");
        let id2 = unshare_process_namespaces(0, CloneFlags::CLONE_NEWPID.bits())
            .expect("second CLONE_NEWPID unshare must succeed");
        assert_ne!(id1, id2, "each unshare must produce a distinct id");
        assert_ne!(id1, 0, "new id must differ from root");
    }

    #[test_case]
    fn test_unshare_unknown_flag_returns_error() {
        let unknown = 0x0000_0001u32; // not a valid CLONE_NEW* bit
        let result = unshare_process_namespaces(0, unknown);
        assert!(
            result.is_err(),
            "unknown flags must be rejected with EINVAL"
        );
    }

    #[test_case]
    fn test_nsfd_setns_roundtrip() {
        ensure_namespace_set(0);
        let ns_id = unshare_process_namespaces(0, CloneFlags::CLONE_NEWNET.bits())
            .expect("CLONE_NEWNET unshare must succeed");
        let fd = nsfd_open(ns_id);
        let result = setns_process_namespaces(0, fd, 0);
        assert!(result.is_ok(), "setns via valid nsfd must succeed");
        nsfd_close(fd);
        // After close, the same fd must fail
        let result2 = setns_process_namespaces(0, fd, 0);
        assert!(result2.is_err(), "setns via closed fd must fail with EBADF");
    }

    #[test_case]
    fn test_setns_invalid_fd_returns_ebadf() {
        let result = setns_process_namespaces(0, -1, 0);
        assert!(result.is_err(), "invalid fd -1 must return EBADF");
    }

    #[test_case]
    fn test_setns_invalid_nstype_returns_einval() {
        ensure_namespace_set(0);
        let ns_id = unshare_process_namespaces(0, CloneFlags::CLONE_NEWUTS.bits())
            .expect("CLONE_NEWUTS unshare must succeed");
        let fd = nsfd_open(ns_id);
        let result = setns_process_namespaces(0, fd, 0x1);
        assert!(result.is_err(), "unknown nstype must return EINVAL");
        nsfd_close(fd);
    }
}
