/// Mount table — manages a hierarchical mount namespace with mount options.
///
/// Supports multiple mount points, mount options (ro, noatime, nosuid, noexec),
/// bind mounts, and mount propagation flags.
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

/// Mount identifier.
pub type MountId = u64;

/// Mount options bit flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountFlags(u32);

impl MountFlags {
    pub const NONE: Self = Self(0);
    pub const RDONLY: Self = Self(1 << 0);
    pub const NOSUID: Self = Self(1 << 1);
    pub const NOEXEC: Self = Self(1 << 2);
    pub const NOATIME: Self = Self(1 << 3);
    pub const NODEV: Self = Self(1 << 4);
    pub const RELATIME: Self = Self(1 << 5);
    pub const SYNCHRONOUS: Self = Self(1 << 6);
    pub const DIRSYNC: Self = Self(1 << 7);
    pub const BIND: Self = Self(1 << 8);

    pub fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }

    pub fn insert(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn remove(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }

    pub fn is_readonly(self) -> bool {
        self.contains(Self::RDONLY)
    }

    pub fn is_nosuid(self) -> bool {
        self.contains(Self::NOSUID)
    }

    pub fn is_noexec(self) -> bool {
        self.contains(Self::NOEXEC)
    }

    pub fn is_noatime(self) -> bool {
        self.contains(Self::NOATIME)
    }
}

impl core::ops::BitOr for MountFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Filesystem type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsType {
    RamFs,
    Ext4,
    Fat32,
    Overlay,
    Devfs,
    Procfs,
    Sysfs,
    Tmpfs,
    Nfs,
    P9,
    Custom(u16),
}

/// A single mount entry.
#[derive(Debug, Clone)]
pub struct MountEntry {
    pub id: MountId,
    /// Parent mount id (0 for root mount).
    pub parent_id: MountId,
    /// Mount point path (normalized, absolute).
    pub mount_point: String,
    /// Filesystem type.
    pub fs_type: FsType,
    /// Source device or path.
    pub source: String,
    /// Mount flags/options.
    pub flags: MountFlags,
    /// Reference count (number of open files under this mount).
    pub ref_count: u32,
}

/// Global mount table.
pub struct MountTable {
    mounts: BTreeMap<MountId, MountEntry>,
    /// Path → mount id index for fast lookup.
    path_index: BTreeMap<String, MountId>,
    next_id: MountId,
}

impl MountTable {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            path_index: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Mount a filesystem at the given path.
    pub fn mount(
        &mut self,
        mount_point: &str,
        source: &str,
        fs_type: FsType,
        flags: MountFlags,
    ) -> Result<MountId, &'static str> {
        let normalized = normalize_path(mount_point);

        // Check for duplicate mount point.
        if self.path_index.contains_key(&normalized) {
            return Err("mount point already in use");
        }

        // Find parent mount.
        let parent_id = self.find_parent_mount(&normalized).unwrap_or(0);

        let id = self.next_id;
        self.next_id += 1;

        let entry = MountEntry {
            id,
            parent_id,
            mount_point: normalized.clone(),
            fs_type,
            source: String::from(source),
            flags,
            ref_count: 0,
        };

        self.mounts.insert(id, entry);
        self.path_index.insert(normalized, id);
        Ok(id)
    }

    /// Unmount a filesystem. Fails if ref_count > 0 (busy).
    pub fn unmount(&mut self, mount_point: &str) -> Result<(), &'static str> {
        let normalized = normalize_path(mount_point);
        let id = self
            .path_index
            .get(&normalized)
            .copied()
            .ok_or("not mounted")?;

        // Check for child mounts.
        let has_children = self
            .mounts
            .values()
            .any(|m| m.parent_id == id && m.id != id);
        if has_children {
            return Err("mount has children; unmount them first");
        }

        let entry = self.mounts.get(&id).ok_or("mount not found")?;
        if entry.ref_count > 0 {
            return Err("mount busy");
        }

        self.mounts.remove(&id);
        self.path_index.remove(&normalized);
        Ok(())
    }

    /// Remount with new flags (e.g., upgrade ro → rw).
    pub fn remount(&mut self, mount_point: &str, flags: MountFlags) -> Result<(), &'static str> {
        let normalized = normalize_path(mount_point);
        let id = self
            .path_index
            .get(&normalized)
            .copied()
            .ok_or("not mounted")?;
        if let Some(entry) = self.mounts.get_mut(&id) {
            entry.flags = flags;
            Ok(())
        } else {
            Err("mount not found")
        }
    }

    /// Resolve the mount for a given path (longest-prefix match).
    pub fn resolve(&self, path: &str) -> Option<&MountEntry> {
        let normalized = normalize_path(path);
        // Find the longest mount prefix.
        let mut best: Option<&MountEntry> = None;
        for entry in self.mounts.values() {
            if normalized.starts_with(&entry.mount_point) {
                match best {
                    Some(b) if b.mount_point.len() >= entry.mount_point.len() => {}
                    _ => best = Some(entry),
                }
            }
        }
        best
    }

    /// Check write permission based on mount flags.
    pub fn check_write(&self, path: &str) -> Result<(), &'static str> {
        if let Some(entry) = self.resolve(path) {
            if entry.flags.is_readonly() {
                return Err("read-only filesystem");
            }
        }
        Ok(())
    }

    /// Increment reference count for a mount.
    pub fn acquire_ref(&mut self, mount_id: MountId) {
        if let Some(entry) = self.mounts.get_mut(&mount_id) {
            entry.ref_count = entry.ref_count.saturating_add(1);
        }
    }

    /// Decrement reference count for a mount.
    pub fn release_ref(&mut self, mount_id: MountId) {
        if let Some(entry) = self.mounts.get_mut(&mount_id) {
            entry.ref_count = entry.ref_count.saturating_sub(1);
        }
    }

    /// Return list of all mounts (like /proc/mounts).
    pub fn list(&self) -> Vec<&MountEntry> {
        self.mounts.values().collect()
    }

    /// Find the parent mount for a given path.
    fn find_parent_mount(&self, path: &str) -> Option<MountId> {
        let mut best: Option<(usize, MountId)> = None;
        for entry in self.mounts.values() {
            if path.starts_with(&entry.mount_point) && path != entry.mount_point {
                match best {
                    Some((len, _)) if len >= entry.mount_point.len() => {}
                    _ => best = Some((entry.mount_point.len(), entry.id)),
                }
            }
        }
        best.map(|(_, id)| id)
    }
}

/// Normalize a path: ensure leading /, collapse //, remove trailing /.
fn normalize_path(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    if !path.starts_with('/') {
        result.push('/');
    }
    let mut prev_slash = false;
    for ch in path.chars() {
        if ch == '/' {
            if !prev_slash {
                result.push('/');
            }
            prev_slash = true;
        } else {
            result.push(ch);
            prev_slash = false;
        }
    }
    // Remove trailing slash (except root).
    if result.len() > 1 && result.ends_with('/') {
        result.pop();
    }
    result
}

/// Global mount table instance.
static GLOBAL_MOUNT_TABLE: Mutex<Option<MountTable>> = Mutex::new(None);

/// Initialize the global mount table.
pub fn init_mount_table() {
    let mut guard = GLOBAL_MOUNT_TABLE.lock();
    if guard.is_none() {
        *guard = Some(MountTable::new());
    }
}

/// Mount a filesystem.
pub fn mount(
    mount_point: &str,
    source: &str,
    fs_type: FsType,
    flags: MountFlags,
) -> Result<MountId, &'static str> {
    let mut guard = GLOBAL_MOUNT_TABLE.lock();
    guard
        .as_mut()
        .ok_or("mount table not initialized")?
        .mount(mount_point, source, fs_type, flags)
}

/// Unmount a filesystem.
pub fn unmount(mount_point: &str) -> Result<(), &'static str> {
    let mut guard = GLOBAL_MOUNT_TABLE.lock();
    guard
        .as_mut()
        .ok_or("mount table not initialized")?
        .unmount(mount_point)
}

/// Resolve mount for a path.
pub fn resolve_mount(path: &str) -> Option<MountId> {
    let guard = GLOBAL_MOUNT_TABLE.lock();
    guard.as_ref().and_then(|mt| mt.resolve(path).map(|e| e.id))
}

/// Check write permission via mount table.
pub fn check_write(path: &str) -> Result<(), &'static str> {
    let guard = GLOBAL_MOUNT_TABLE.lock();
    match guard.as_ref() {
        Some(mt) => mt.check_write(path),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn resolve_prefers_longest_mount_prefix() {
        let mut table = MountTable::new();
        let root = table
            .mount("/", "rootfs", FsType::RamFs, MountFlags::NONE)
            .unwrap();
        let nested = table
            .mount("/srv/data", "datafs", FsType::Ext4, MountFlags::RDONLY)
            .unwrap();

        assert_eq!(
            table.resolve("/srv/data/file.txt").map(|entry| entry.id),
            Some(nested)
        );
        assert_eq!(
            table.resolve("/etc/hosts").map(|entry| entry.id),
            Some(root)
        );
    }

    #[test_case]
    fn unmount_rejects_busy_and_child_mounts() {
        let mut table = MountTable::new();
        let parent = table
            .mount("/mnt", "ram", FsType::RamFs, MountFlags::NONE)
            .unwrap();
        let _child = table
            .mount("/mnt/nested", "nested", FsType::Tmpfs, MountFlags::NONE)
            .unwrap();
        assert_eq!(
            table.unmount("/mnt"),
            Err("mount has children; unmount them first")
        );

        let mut busy = MountTable::new();
        let busy_id = busy
            .mount("/busy", "ram", FsType::RamFs, MountFlags::NONE)
            .unwrap();
        busy.acquire_ref(busy_id);
        assert_eq!(busy.unmount("/busy"), Err("mount busy"));
        busy.release_ref(busy_id);
        assert_eq!(busy.unmount("/busy"), Ok(()));

        let _ = parent;
    }

    #[test_case]
    fn remount_updates_write_permissions_and_normalizes_paths() {
        let mut table = MountTable::new();
        table
            .mount("//var//log//", "ram", FsType::RamFs, MountFlags::NONE)
            .unwrap();
        assert_eq!(table.check_write("/var/log/messages"), Ok(()));
        table
            .remount("/var/log", MountFlags::RDONLY | MountFlags::NOEXEC)
            .unwrap();
        assert_eq!(
            table.check_write("/var/log/messages"),
            Err("read-only filesystem")
        );
        let entry = table.resolve("/var/log/messages").unwrap();
        assert!(entry.flags.is_noexec());
        assert_eq!(entry.mount_point, "/var/log");
    }

    #[test_case]
    fn mount_rejects_duplicate_normalized_paths_and_tracks_parent_mount() {
        let mut table = MountTable::new();
        let root = table
            .mount("/", "rootfs", FsType::RamFs, MountFlags::NONE)
            .unwrap();
        let child = table
            .mount("/srv//logs/", "logs", FsType::Tmpfs, MountFlags::NONE)
            .unwrap();

        assert_eq!(
            table.mount("/srv/logs", "dup", FsType::Tmpfs, MountFlags::NONE),
            Err("mount point already exists")
        );
        assert_eq!(
            table.mounts.get(&child).map(|entry| entry.parent_id),
            Some(root)
        );
    }

    #[test_case]
    fn resolve_does_not_match_partial_path_components() {
        let mut table = MountTable::new();
        let root = table
            .mount("/", "rootfs", FsType::RamFs, MountFlags::NONE)
            .unwrap();
        let srv = table
            .mount("/srv", "srvfs", FsType::Tmpfs, MountFlags::NONE)
            .unwrap();

        assert_eq!(table.resolve("/srv/data").map(|entry| entry.id), Some(srv));
        assert_eq!(
            table.resolve("/srvx/file").map(|entry| entry.id),
            Some(root)
        );
    }
}
