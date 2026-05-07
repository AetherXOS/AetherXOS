use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use super::types::{CloneFlags, NsId, NsType, NS_CREATES, NS_SETNS_CALLS, NS_UNSHARE_CALLS};
use super::*;

/// The full set of namespaces associated with a process.
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
