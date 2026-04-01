/// PID Namespace — isolated PID number spaces.
///
/// Each PID namespace has its own PID counter starting at 1.
/// PID 1 inside the namespace is the init process for that namespace.
/// A process is visible in its own namespace and all ancestor namespaces
/// (with different PIDs at each level).
use super::{alloc_ns_id, NsId};
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

/// A single PID namespace.
pub struct PidNamespace {
    pub id: NsId,
    /// Next PID to allocate within this namespace.
    next_pid: AtomicU32,
    /// Depth in the namespace hierarchy (root = 0).
    pub depth: u32,
    /// PID of the init process (first process in this namespace).
    pub init_pid: u32,
    /// Mapping from namespace-local PID → global (kernel) PID.
    pid_map: BTreeMap<u32, u64>,
}

impl PidNamespace {
    /// Create the root PID namespace.
    pub fn root() -> Self {
        Self {
            id: alloc_ns_id(),
            next_pid: AtomicU32::new(1),
            depth: 0,
            init_pid: 1,
            pid_map: BTreeMap::new(),
        }
    }

    /// Create a child PID namespace.
    pub fn child_of(parent: &PidNamespace) -> Self {
        Self {
            id: alloc_ns_id(),
            next_pid: AtomicU32::new(1),
            depth: parent.depth + 1,
            init_pid: 0,
            pid_map: BTreeMap::new(),
        }
    }

    /// Allocate a new PID in this namespace, mapped to `global_pid`.
    pub fn alloc_pid(&mut self, global_pid: u64) -> u32 {
        let local = self.next_pid.fetch_add(1, Ordering::Relaxed);
        self.pid_map.insert(local, global_pid);
        if local == 1 {
            self.init_pid = 1;
        }
        local
    }

    /// Release a PID.
    pub fn free_pid(&mut self, local_pid: u32) {
        self.pid_map.remove(&local_pid);
    }

    /// Translate a namespace-local PID to a global PID.
    pub fn to_global(&self, local_pid: u32) -> Option<u64> {
        self.pid_map.get(&local_pid).copied()
    }

    /// Translate a global PID to namespace-local PID.
    pub fn from_global(&self, global_pid: u64) -> Option<u32> {
        for (&local, &global) in &self.pid_map {
            if global == global_pid {
                return Some(local);
            }
        }
        None
    }

    /// Number of active PIDs in this namespace.
    pub fn active_count(&self) -> usize {
        self.pid_map.len()
    }
}
