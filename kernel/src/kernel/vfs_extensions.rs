// --- PHASE 5: VFS EXTENSIONS ---
// File permissions, mount management, quotas

use crate::core::log;
use alloc::format;
use crate::interfaces::vfs_ext::{
    FileOwner, FilePermissionManager, FilePermissions, MountInfo, MountManager,
    QuotaInfo, QuotaManager,
};
use alloc::collections::BTreeMap;
use alloc::string::String;
use crate::kernel::sync::IrqSafeMutex;

/// File permission bits and flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcreteFilePermissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_exec: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_exec: bool,
    pub others_read: bool,
    pub others_write: bool,
    pub others_exec: bool,
    pub setuid: bool,
    pub setgid: bool,
    pub sticky: bool,
}

impl ConcreteFilePermissions {
    /// Convert to octal notation (e.g., 0755)
    pub fn to_octal(&self) -> u32 {
        let mut value = 0u32;

        if self.owner_read {
            value |= 0o400;
        }
        if self.owner_write {
            value |= 0o200;
        }
        if self.owner_exec {
            value |= 0o100;
        }

        if self.group_read {
            value |= 0o040;
        }
        if self.group_write {
            value |= 0o020;
        }
        if self.group_exec {
            value |= 0o010;
        }

        if self.others_read {
            value |= 0o004;
        }
        if self.others_write {
            value |= 0o002;
        }
        if self.others_exec {
            value |= 0o001;
        }

        if self.setuid {
            value |= 0o4000;
        }
        if self.setgid {
            value |= 0o2000;
        }
        if self.sticky {
            value |= 0o1000;
        }

        value
    }

    /// Create from octal notation
    pub fn from_octal(mode: u32) -> Self {
        Self {
            owner_read: (mode & 0o400) != 0,
            owner_write: (mode & 0o200) != 0,
            owner_exec: (mode & 0o100) != 0,
            group_read: (mode & 0o040) != 0,
            group_write: (mode & 0o020) != 0,
            group_exec: (mode & 0o010) != 0,
            others_read: (mode & 0o004) != 0,
            others_write: (mode & 0o002) != 0,
            others_exec: (mode & 0o001) != 0,
            setuid: (mode & 0o4000) != 0,
            setgid: (mode & 0o2000) != 0,
            sticky: (mode & 0o1000) != 0,
        }
    }

    /// Check if owner can read
    pub fn can_owner_read(&self) -> bool {
        self.owner_read
    }

    /// Check if owner can write
    pub fn can_owner_write(&self) -> bool {
        self.owner_write
    }

    /// Check if owner can execute
    pub fn can_owner_exec(&self) -> bool {
        self.owner_exec
    }
}

/// Concrete file permission manager
pub struct ConcreteFilePermissionManager {
    /// File → permissions mapping
    permissions: IrqSafeMutex<BTreeMap<u64, FilePermissions>>,

    /// File → owner mapping
    owners: IrqSafeMutex<BTreeMap<u64, FileOwner>>,
}

impl ConcreteFilePermissionManager {
    /// Create a new permission manager
    pub const fn new() -> Self {
        Self {
            permissions: IrqSafeMutex::new(BTreeMap::new()),
            owners: IrqSafeMutex::new(BTreeMap::new()),
        }
    }
}

impl FilePermissionManager for ConcreteFilePermissionManager {
    /// Check if access is allowed
    fn check_permission(
        &self,
        inode: u64,
        permission: FilePermissions,
        uid: u32,
        gid: u32,
    ) -> crate::interfaces::KernelResult<bool> {
        let owners = self.owners.lock();
        let perms_map = self.permissions.lock();
        
        let owner = owners.get(&inode).ok_or(crate::interfaces::KernelError::NotFound)?;
        let perms = perms_map.get(&inode).ok_or(crate::interfaces::KernelError::NotFound)?;

        let is_owner = owner.uid == uid;
        let is_group = owner.gid == gid;

        // Simplified check
        if permission.can_read(is_owner, is_group) && !perms.can_read(is_owner, is_group) {
            return Ok(false);
        }
        if permission.can_write(is_owner, is_group) && !perms.can_write(is_owner, is_group) {
            return Ok(false);
        }
        if permission.can_execute(is_owner, is_group) && !perms.can_execute(is_owner, is_group) {
            return Ok(false);
        }

        Ok(true)
    }

    /// Set file permissions
    fn set_permissions(&self, inode: u64, perms: FilePermissions) -> crate::interfaces::KernelResult<()> {
        self.permissions.lock().insert(inode, perms);
        log::debug(&format!(
            "Inode {} permissions set",
            inode
        ));
        Ok(())
    }

    /// Get file permissions
    fn get_permissions(&self, inode: u64) -> crate::interfaces::KernelResult<FilePermissions> {
        self.permissions
            .lock()
            .get(&inode)
            .copied()
            .ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Set file owner
    fn set_owner(&self, inode: u64, owner: FileOwner) -> crate::interfaces::KernelResult<()> {
        self.owners.lock().insert(inode, owner);
        log::debug(&format!(
            "Inode {} owner set to: UID={} GID={}",
            inode, owner.uid, owner.gid
        ));
        Ok(())
    }

    /// Get file owner
    fn get_owner(&self, inode: u64) -> crate::interfaces::KernelResult<FileOwner> {
        self.owners.lock().get(&inode).copied().ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Change owner (chown)
    fn chown(&self, inode: u64, uid: u32, gid: u32, _caller_uid: u32) -> crate::interfaces::KernelResult<()> {
        self.owners.lock().insert(
            inode,
            FileOwner { uid, gid },
        );
        log::debug(&format!(
            "Inode {} chown: UID={} GID={}",
            inode, uid, gid
        ));
        Ok(())
    }

    /// Change permissions (chmod)
    fn chmod(&self, inode: u64, mode: u16, _caller_uid: u32) -> crate::interfaces::KernelResult<()> {
        let perms = FilePermissions {
            owner: ((mode >> 6) & 0o7) as u8,
            group: ((mode >> 3) & 0o7) as u8,
            others: (mode & 0o7) as u8,
            setuid: (mode & 0o4000) != 0,
            setgid: (mode & 0o2000) != 0,
            sticky: (mode & 0o1000) != 0,
        };
        self.permissions.lock().insert(inode, perms);
        log::debug(&format!("Inode {} chmod: {:#o}", inode, mode));
        Ok(())
    }
}

/// Mount point information
#[derive(Debug, Clone)]
pub struct ConcreteMountInfo {
    pub mount_path: String,
    pub filesystem_type: String,
    pub source: String,
    pub options: String,
    pub read_only: bool,
}

/// Concrete mount manager
pub struct ConcreteMountManager {
    /// Mount points
    mounts: IrqSafeMutex<BTreeMap<String, ConcreteMountInfo>>,
}

impl ConcreteMountManager {
    /// Create a new mount manager
    pub const fn new() -> Self {
        Self {
            mounts: IrqSafeMutex::new(BTreeMap::new()),
        }
    }
}

impl MountManager for ConcreteMountManager {
    /// Mount a filesystem
    fn mount(
        &self,
        mount_path: &str,
        filesystem_type: &str,
        source: &str,
        read_only: bool,
    ) -> crate::interfaces::KernelResult<()> {
        let mount = ConcreteMountInfo {
            mount_path: mount_path.into(),
            filesystem_type: filesystem_type.into(),
            source: source.into(),
            options: "".into(),
            read_only,
        };

        self.mounts
            .lock()
            .insert(mount_path.into(), mount);
        log::info(&format!(
            "Mounted {} ({}) at {}",
            source, filesystem_type, mount_path
        ));
        Ok(())
    }

    /// Unmount a filesystem
    fn unmount(&self, mount_path: &str) -> crate::interfaces::KernelResult<()> {
        if self.mounts.lock().remove(mount_path).is_some() {
            log::info(&format!("Unmounted {}", mount_path));
            Ok(())
        } else {
            Err(crate::interfaces::KernelError::NotFound)
        }
    }

    /// Get mount information
    fn get_mount_info(&self, mount_path: &str) -> crate::interfaces::KernelResult<MountInfo> {
        self.mounts.lock().get(mount_path)
            .map(|m| MountInfo {
                mount_path: m.mount_path.clone(),
                filesystem_type: m.filesystem_type.clone(),
                source: m.source.clone(),
                options: alloc::vec![m.options.clone()],
                read_only: m.read_only,
            })
            .ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// List all mounts
    fn list_mounts(&self) -> alloc::vec::Vec<MountInfo> {
        self.mounts
            .lock()
            .values()
            .map(|m| MountInfo {
                mount_path: m.mount_path.clone(),
                filesystem_type: m.filesystem_type.clone(),
                source: m.source.clone(),
                options: alloc::vec![m.options.clone()],
                read_only: m.read_only,
            })
            .collect()
    }

    /// Check if path is mounted
    fn is_mounted(&self, mount_path: &str) -> bool {
        self.mounts.lock().contains_key(mount_path)
    }
}

/// Concrete quota manager
pub struct ConcreteQuotaManager {
    /// Per-user quota
    user_quotas: IrqSafeMutex<BTreeMap<u32, QuotaInfo>>,

    /// Per-group quota
    group_quotas: IrqSafeMutex<BTreeMap<u32, QuotaInfo>>,
}

impl ConcreteQuotaManager {
    /// Create a new quota manager
    pub const fn new() -> Self {
        Self {
            user_quotas: IrqSafeMutex::new(BTreeMap::new()),
            group_quotas: IrqSafeMutex::new(BTreeMap::new()),
        }
    }
}

impl QuotaManager for ConcreteQuotaManager {
    /// Set block quota for user
    fn set_block_quota(&self, user_id: u32, limit_blocks: u64) -> crate::interfaces::KernelResult<()> {
        let mut quotas = self.user_quotas.lock();
        let quota = quotas.entry(user_id)
            .or_insert_with(|| QuotaInfo {
                used_blocks: 0,
                block_limit: 0,
                used_inodes: 0,
                inode_limit: 0,
            });
        quota.block_limit = limit_blocks;
        log::debug(&format!(
            "User {} block quota set to {}",
            user_id, limit_blocks
        ));
        Ok(())
    }

    /// Set inode quota for user
    fn set_inode_quota(&self, user_id: u32, limit_inodes: u64) -> crate::interfaces::KernelResult<()> {
        let mut quotas = self.user_quotas.lock();
        let quota = quotas.entry(user_id)
            .or_insert_with(|| QuotaInfo {
                used_blocks: 0,
                block_limit: 0,
                used_inodes: 0,
                inode_limit: 0,
            });
        quota.inode_limit = limit_inodes;
        log::debug(&format!(
            "User {} inode quota set to {}",
            user_id, limit_inodes
        ));
        Ok(())
    }

    /// Get quota for user
    fn get_quota(&self, user_id: u32) -> crate::interfaces::KernelResult<QuotaInfo> {
        self.user_quotas.lock().get(&user_id).copied().ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Check if user can allocate more space
    fn can_allocate(&self, user_id: u32, blocks: u64) -> crate::interfaces::KernelResult<bool> {
        if let Some(quota) = self.user_quotas.lock().get(&user_id) {
            Ok(quota.used_blocks + blocks <= quota.block_limit)
        } else {
            Ok(true) // No quota set
        }
    }

    /// Get quota report for user
    fn report_quota(&self, user_id: u32) -> String {
        if let Some(quota) = self.user_quotas.lock().get(&user_id) {
            format!("Quota for {}: {}/{} blocks, {}/{} inodes", 
                user_id, quota.used_blocks, quota.block_limit, quota.used_inodes, quota.inode_limit)
        } else {
            format!("No quota set for {}", user_id)
        }
    }
}

// Global instances
pub static PERMISSION_MANAGER: ConcreteFilePermissionManager = ConcreteFilePermissionManager::new();
pub static MOUNT_MANAGER: ConcreteMountManager = ConcreteMountManager::new();
pub static QUOTA_MANAGER: ConcreteQuotaManager = ConcreteQuotaManager::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_permissions_octal() {
        let perms = FilePermissions {
            owner: 0o7,
            group: 0o5,
            others: 0o4,
            setuid: false,
            setgid: false,
            sticky: false,
        };
        assert_eq!(perms.to_octal(), 0o754);
    }

    #[test]
    fn test_permission_manager_set_perms() {
        let mgr = ConcreteFilePermissionManager::new();
        let perms = FilePermissions {
            owner: 0o6,
            group: 0o4,
            others: 0o4,
            setuid: false,
            setgid: false,
            sticky: false,
        };
        assert!(mgr.set_permissions(1, perms).is_ok());
        assert_eq!(mgr.get_permissions(1).unwrap(), perms);
    }

    #[test]
    fn test_permission_manager_owner() {
        let mgr = ConcreteFilePermissionManager::new();
        let owner = FileOwner { uid: 1000, gid: 1000 };
        assert!(mgr.set_owner(1, owner).is_ok());
        assert_eq!(mgr.get_owner(1).unwrap(), owner);
    }

    #[test]
    fn test_mount_manager_mount() {
        let mgr = ConcreteMountManager::new();
        assert!(mgr.mount("/", "ext4", "/dev/sda1", false).is_ok());
        assert!(mgr.is_mounted("/"));
    }

    #[test]
    fn test_mount_manager_list() {
        let mgr = ConcreteMountManager::new();
        mgr.mount("/", "ext4", "/dev/sda1", false).ok();
        mgr.mount("/boot", "ext4", "/dev/sda2", false).ok();

        let mounts = mgr.list_mounts();
        assert_eq!(mounts.len(), 2);
    }

    #[test]
    fn test_quota_manager() {
        let mgr = ConcreteQuotaManager::new();
        assert!(mgr.set_block_quota(1000, 1_000_000).is_ok());
        assert!(mgr.set_inode_quota(1000, 100_000).is_ok());

        let quota = mgr.get_quota(1000).unwrap();
        assert_eq!(quota.block_limit, 1_000_000);
    }

    #[test]
    fn test_quota_enforcement() {
        let mgr = ConcreteQuotaManager::new();
        mgr.set_block_quota(1000, 1000).ok();

        assert!(mgr.can_allocate(1000, 500).unwrap());
        assert!(!mgr.can_allocate(1000, 1500).unwrap());
    }
}
