/// User Namespace — UID/GID mapping and privilege isolation.
///
/// Processes in a user namespace have a mapping from namespace-local
/// UIDs/GIDs to host (parent namespace) UIDs/GIDs.  A process can be
/// root (UID 0) inside the namespace while being unprivileged on the host.
use super::{alloc_ns_id, NsId};
use alloc::vec::Vec;

/// A single UID/GID mapping entry (like /proc/PID/uid_map).
#[derive(Debug, Clone, Copy)]
pub struct IdMapEntry {
    /// Start of the ID range inside the namespace.
    pub ns_id: u32,
    /// Start of the ID range on the host.
    pub host_id: u32,
    /// Length of the range.
    pub count: u32,
}

/// A single user namespace.
pub struct UserNamespace {
    pub id: NsId,
    uid_map: Vec<IdMapEntry>,
    gid_map: Vec<IdMapEntry>,
    /// Depth in the user namespace hierarchy (root = 0).
    pub depth: u32,
}

impl UserNamespace {
    /// Create the root (init) user namespace.
    /// Identity mapping: ns 0 → host 0 for all 65536 IDs.
    pub fn root() -> Self {
        Self {
            id: alloc_ns_id(),
            uid_map: alloc::vec![IdMapEntry {
                ns_id: 0,
                host_id: 0,
                count: 65536,
            }],
            gid_map: alloc::vec![IdMapEntry {
                ns_id: 0,
                host_id: 0,
                count: 65536,
            }],
            depth: 0,
        }
    }

    /// Create a new (empty) user namespace.  No mappings by default;
    /// `set_uid_map` / `set_gid_map` must be called before use.
    pub fn new() -> Self {
        Self {
            id: alloc_ns_id(),
            uid_map: Vec::new(),
            gid_map: Vec::new(),
            depth: 0,
        }
    }

    /// Set the UID mapping (max 5 entries, per Linux convention).
    pub fn set_uid_map(&mut self, entries: Vec<IdMapEntry>) -> bool {
        if entries.len() > 5 || !self.uid_map.is_empty() {
            return false; // Already written or too many entries.
        }
        self.uid_map = entries;
        true
    }

    /// Set the GID mapping.
    pub fn set_gid_map(&mut self, entries: Vec<IdMapEntry>) -> bool {
        if entries.len() > 5 || !self.gid_map.is_empty() {
            return false;
        }
        self.gid_map = entries;
        true
    }

    /// Translate a namespace-local UID to a host UID.
    pub fn ns_uid_to_host(&self, ns_uid: u32) -> Option<u32> {
        for entry in &self.uid_map {
            if ns_uid >= entry.ns_id && ns_uid < entry.ns_id + entry.count {
                return Some(entry.host_id + (ns_uid - entry.ns_id));
            }
        }
        None // unmapped → overflow UID (65534)
    }

    /// Translate a host UID to namespace-local UID.
    pub fn host_uid_to_ns(&self, host_uid: u32) -> Option<u32> {
        for entry in &self.uid_map {
            if host_uid >= entry.host_id && host_uid < entry.host_id + entry.count {
                return Some(entry.ns_id + (host_uid - entry.host_id));
            }
        }
        None
    }

    /// Translate a namespace-local GID to a host GID.
    pub fn ns_gid_to_host(&self, ns_gid: u32) -> Option<u32> {
        for entry in &self.gid_map {
            if ns_gid >= entry.ns_id && ns_gid < entry.ns_id + entry.count {
                return Some(entry.host_id + (ns_gid - entry.ns_id));
            }
        }
        None
    }

    /// Translate a host GID to namespace-local GID.
    pub fn host_gid_to_ns(&self, host_gid: u32) -> Option<u32> {
        for entry in &self.gid_map {
            if host_gid >= entry.host_id && host_gid < entry.host_id + entry.count {
                return Some(entry.ns_id + (host_gid - entry.host_id));
            }
        }
        None
    }

    /// Check if the namespace has any UID mapping.
    pub fn has_uid_map(&self) -> bool {
        !self.uid_map.is_empty()
    }

    /// Check if the namespace has any GID mapping.
    pub fn has_gid_map(&self) -> bool {
        !self.gid_map.is_empty()
    }
}
