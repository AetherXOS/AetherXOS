/// Mount Namespace — per-process mount table view.
///
/// Each mount namespace holds an independent snapshot of the mount hierarchy.
/// `unshare(CLONE_NEWNS)` creates a copy of the parent's mount table;
/// subsequent mounts/unmounts in one namespace are not visible in others.
use super::{alloc_ns_id, NsId};
use alloc::string::String;
use alloc::vec::Vec;

/// An entry in the namespace's mount table.
#[derive(Debug, Clone)]
pub struct NsMountEntry {
    pub mount_id: u64,
    pub source: String,
    pub target: String,
    pub fs_type: String,
    pub flags: u32,
    pub readonly: bool,
}

/// A single mount namespace.
pub struct MountNamespace {
    pub id: NsId,
    mounts: Vec<NsMountEntry>,
    next_mount_id: u64,
}

impl MountNamespace {
    /// Create the root mount namespace.
    pub fn root() -> Self {
        Self {
            id: alloc_ns_id(),
            mounts: Vec::new(),
            next_mount_id: 1,
        }
    }

    /// Clone this mount namespace (copy-on-fork semantics).
    pub fn clone_from(parent: &MountNamespace) -> Self {
        Self {
            id: alloc_ns_id(),
            mounts: parent.mounts.clone(),
            next_mount_id: parent.next_mount_id,
        }
    }

    /// Add a mount entry.
    pub fn mount(&mut self, source: String, target: String, fs_type: String, flags: u32) -> u64 {
        let id = self.next_mount_id;
        self.next_mount_id += 1;
        self.mounts.push(NsMountEntry {
            mount_id: id,
            source,
            target,
            fs_type,
            flags,
            readonly: flags & 0x1 != 0, // MS_RDONLY
        });
        id
    }

    /// Unmount by target path. Returns true if found and removed.
    pub fn umount(&mut self, target: &str) -> bool {
        if let Some(pos) = self.mounts.iter().position(|m| m.target == target) {
            self.mounts.remove(pos);
            true
        } else {
            false
        }
    }

    /// Lookup mount by target path (longest-prefix match).
    pub fn find_mount(&self, path: &str) -> Option<&NsMountEntry> {
        let mut best: Option<&NsMountEntry> = None;
        let mut best_len = 0;
        for m in &self.mounts {
            if path.starts_with(&m.target) && m.target.len() > best_len {
                best = Some(m);
                best_len = m.target.len();
            }
        }
        best
    }

    /// List all mounts.
    pub fn list_mounts(&self) -> &[NsMountEntry] {
        &self.mounts
    }

    /// Number of mounts.
    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }

    /// Set a mount as read-only.
    pub fn set_readonly(&mut self, target: &str, readonly: bool) -> bool {
        for m in &mut self.mounts {
            if m.target == target {
                m.readonly = readonly;
                return true;
            }
        }
        false
    }
}
