use core::sync::atomic::{AtomicU64, Ordering};

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

/// Allocate a globally unique namespace ID.
pub fn alloc_ns_id() -> NsId {
    NEXT_NS_ID.fetch_add(1, Ordering::Relaxed)
}

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

pub static NS_CREATES: AtomicU64 = AtomicU64::new(0);
pub static NS_DESTROYS: AtomicU64 = AtomicU64::new(0);
pub static NS_UNSHARE_CALLS: AtomicU64 = AtomicU64::new(0);
pub static NS_SETNS_CALLS: AtomicU64 = AtomicU64::new(0);

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

pub fn namespace_flag_mask() -> u32 {
    CloneFlags::CLONE_NEWPID.bits()
        | CloneFlags::CLONE_NEWNET.bits()
        | CloneFlags::CLONE_NEWNS.bits()
        | CloneFlags::CLONE_NEWIPC.bits()
        | CloneFlags::CLONE_NEWUTS.bits()
        | CloneFlags::CLONE_NEWUSER.bits()
        | CloneFlags::CLONE_NEWCGROUP.bits()
}

pub fn nstype_to_ns_type(nstype: u32) -> Option<NsType> {
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
