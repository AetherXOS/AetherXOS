/// VFS extension interfaces.
/// 
/// Advanced filesystem capabilities: permissions, mounts, quotas,
/// and mount point management.

use crate::interfaces::KernelResult;
use alloc::string::String;

/// File permission bits (Unix-like)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    /// Owner permissions (read, write, execute)
    pub owner: u8,
    /// Group permissions
    pub group: u8,
    /// Others permissions
    pub others: u8,
    /// Set UID bit
    pub setuid: bool,
    /// Set GID bit
    pub setgid: bool,
    /// Sticky bit
    pub sticky: bool,
}

impl FilePermissions {
    /// Convert to traditional octal representation (e.g., 0755)
    pub fn to_octal(&self) -> u16 {
        let mut result = 0u16;
        if self.setuid {
            result |= 0o4000;
        }
        if self.setgid {
            result |= 0o2000;
        }
        if self.sticky {
            result |= 0o1000;
        }
        result |= ((self.owner as u16) << 6) | ((self.group as u16) << 3) | (self.others as u16);
        result
    }

    /// Create permissions from octal mode (e.g., 0755)
    pub fn from_octal(mode: u16) -> Self {
        Self {
            owner: ((mode >> 6) & 0o7) as u8,
            group: ((mode >> 3) & 0o7) as u8,
            others: (mode & 0o7) as u8,
            setuid: (mode & 0o4000) != 0,
            setgid: (mode & 0o2000) != 0,
            sticky: (mode & 0o1000) != 0,
        }
    }

    /// Create standard RWX permissions from a combined mode
    pub fn new_rwx(mode: u16) -> Self {
        Self::from_octal(mode)
    }

    /// Check read permission for user
    pub fn can_read(&self, is_owner: bool, is_group: bool) -> bool {
        if is_owner {
            (self.owner & 0o4) != 0
        } else if is_group {
            (self.group & 0o4) != 0
        } else {
            (self.others & 0o4) != 0
        }
    }

    /// Check write permission for user
    pub fn can_write(&self, is_owner: bool, is_group: bool) -> bool {
        if is_owner {
            (self.owner & 0o2) != 0
        } else if is_group {
            (self.group & 0o2) != 0
        } else {
            (self.others & 0o2) != 0
        }
    }

    /// Check execute permission for user
    pub fn can_execute(&self, is_owner: bool, is_group: bool) -> bool {
        if is_owner {
            (self.owner & 0o1) != 0
        } else if is_group {
            (self.group & 0o1) != 0
        } else {
            (self.others & 0o1) != 0
        }
    }
}

/// File ownership information
#[derive(Debug, Clone, Copy)]
pub struct FileOwner {
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
}

/// Mount point information
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Mount path (e.g., "/mnt/usb")
    pub mount_path: String,
    /// Filesystem type (e.g., "ext4", "vfat")
    pub filesystem_type: String,
    /// Device or image path
    pub source: String,
    /// Mount options
    pub options: alloc::vec::Vec<String>,
    /// Is read-only?
    pub read_only: bool,
}

/// Trait for file permission management
pub trait FilePermissionManager: Send + Sync {
    /// Check if user can perform action
    fn check_permission(
        &self,
        inode_id: u64,
        permission: FilePermissions,
        uid: u32,
        gid: u32,
    ) -> KernelResult<bool>;

    /// Set file permissions
    fn set_permissions(&self, inode_id: u64, permissions: FilePermissions) -> KernelResult<()>;

    /// Get file permissions
    fn get_permissions(&self, inode_id: u64) -> KernelResult<FilePermissions>;

    /// Set file owner
    fn set_owner(&self, inode_id: u64, owner: FileOwner) -> KernelResult<()>;

    /// Get file owner
    fn get_owner(&self, inode_id: u64) -> KernelResult<FileOwner>;

    /// Change owner (with elevated privileges check)
    fn chown(&self, inode_id: u64, uid: u32, gid: u32, caller_uid: u32) -> KernelResult<()>;

    /// Change permissions (with elevated privileges check)
    fn chmod(&self, inode_id: u64, mode: u16, caller_uid: u32) -> KernelResult<()>;
}

/// Trait for mount point management
pub trait MountManager: Send + Sync {
    /// Mount a filesystem
    fn mount(
        &self,
        mount_path: &str,
        filesystem_type: &str,
        source: &str,
        read_only: bool,
    ) -> KernelResult<()>;

    /// Unmount a filesystem
    fn unmount(&self, mount_path: &str) -> KernelResult<()>;

    /// Get mount info for path
    fn get_mount_info(&self, path: &str) -> KernelResult<MountInfo>;

    /// List all mounts
    fn list_mounts(&self) -> alloc::vec::Vec<MountInfo>;

    /// Check if mount is active
    fn is_mounted(&self, mount_path: &str) -> bool;
}

/// File quota information
#[derive(Debug, Clone, Copy)]
pub struct QuotaInfo {
    /// Total blocks allocated to user/group
    pub used_blocks: u64,
    /// Block quota limit
    pub block_limit: u64,
    /// Total inodes used
    pub used_inodes: u64,
    /// Inode quota limit
    pub inode_limit: u64,
}

impl QuotaInfo {
    /// Check if block quota exceeded
    pub fn block_exceeded(&self) -> bool {
        self.used_blocks > self.block_limit
    }

    /// Check if inode quota exceeded
    pub fn inode_exceeded(&self) -> bool {
        self.used_inodes > self.inode_limit
    }

    /// Get block usage percentage
    pub fn block_usage_percent(&self) -> f32 {
        if self.block_limit == 0 {
            100.0
        } else {
            (self.used_blocks as f32 / self.block_limit as f32) * 100.0
        }
    }
}

/// Trait for filesystem quota management
pub trait QuotaManager: Send + Sync {
    /// Set block quota for user
    fn set_block_quota(&self, uid: u32, limit: u64) -> KernelResult<()>;

    /// Set inode quota for user
    fn set_inode_quota(&self, uid: u32, limit: u64) -> KernelResult<()>;

    /// Get quota info for user
    fn get_quota(&self, uid: u32) -> KernelResult<QuotaInfo>;

    /// Check if allocation would exceed quota
    fn can_allocate(&self, uid: u32, blocks: u64) -> KernelResult<bool>;

    /// Report quota to user
    fn report_quota(&self, uid: u32) -> alloc::string::String;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_permissions_octal() {
        let perms = FilePermissions {
            owner: 0o7,
            group: 0o5,
            others: 0o5,
            setuid: false,
            setgid: false,
            sticky: false,
        };
        assert_eq!(perms.to_octal(), 0o755);
    }

    #[test]
    fn test_permission_checks() {
        let perms = FilePermissions {
            owner: 0o6,  // rw-
            group: 0o4,  // r--
            others: 0o0, // ---
            setuid: false,
            setgid: false,
            sticky: false,
        };

        // Owner checks
        assert!(perms.can_read(true, false));
        assert!(perms.can_write(true, false));
        assert!(!perms.can_execute(true, false));

        // Group checks
        assert!(perms.can_read(false, true));
        assert!(!perms.can_write(false, true));
        assert!(!perms.can_execute(false, true));

        // Others checks
        assert!(!perms.can_read(false, false));
        assert!(!perms.can_write(false, false));
        assert!(!perms.can_execute(false, false));
    }

    #[test]
    fn test_quota_usage() {
        let quota = QuotaInfo {
            used_blocks: 500,
            block_limit: 1000,
            used_inodes: 200,
            inode_limit: 500,
        };

        assert!(!quota.block_exceeded());
        assert!(!quota.inode_exceeded());
        assert!((quota.block_usage_percent() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_quota_exceeded() {
        let quota = QuotaInfo {
            used_blocks: 1500,
            block_limit: 1000,
            used_inodes: 200,
            inode_limit: 500,
        };

        assert!(quota.block_exceeded());
        assert!(!quota.inode_exceeded());
    }
}
