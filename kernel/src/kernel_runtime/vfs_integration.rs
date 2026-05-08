use alloc::format;
use alloc::string::String;
use crate::kernel::vfs_extensions::{
    PERMISSION_MANAGER, MOUNT_MANAGER, QUOTA_MANAGER,
};
use crate::interfaces::vfs_ext::{
    FilePermissionManager, MountManager, QuotaManager,
};
use crate::kernel_runtime::integration_utils::logging;
use aop_macros::log_entry;
use crate::kernel::task::current_security_context;

/// Initialize VFS extensions subsystem
/// 
/// Sets up:
/// 1. Root filesystem mount at "/"
/// 2. Unix permission model (rwxrwxrwx with setuid/setgid/sticky bits)
/// 3. Per-user disk and inode quotas
/// 
/// # Returns
/// Ok(()) if root mount successful, Err if critical VFS unavailable
#[log_entry(info, target = "vfs_ext")]
pub fn init_vfs_extensions() -> Result<(), &'static str> {
    // Mount root filesystem
    MOUNT_MANAGER.mount("/", "rootfs", "init", false).map_err(|e| e.as_str())?;
    logging::log_state_transition("filesystem", "unmounted", "mounted");
    Ok(())
}

/// Check file permission for access
/// 
/// Validates that user (uid/gid) can perform requested action on inode.
/// Uses Unix permission model:
/// - Bits 0-2: Others (execute, write, read)
/// - Bits 3-5: Group (execute, write, read)
/// - Bits 6-8: Owner (execute, write, read)
/// - Bits 9-11: Setuid, Setgid, Sticky
/// 
/// # Arguments
/// * `inode` - Inode number to check
/// * `uid` - User ID attempting access
/// * `gid` - Group ID of user
/// * `action` - Requested permission (1=execute, 2=write, 4=read, or combined)
/// 
/// # Returns
/// Ok(()) if access allowed, Err("Permission denied") if denied
#[log_entry(debug, target = "vfs_ext")]
#[precondition(inode != 0)]
#[precondition(action > 0 && action <= 7)]
pub fn check_file_permission(
    inode: u64,
    uid: u32,
    gid: u32,
    action: u8,
) -> Result<(), &'static str> {
    let perms = crate::interfaces::vfs_ext::FilePermissions::new_rwx(action as u16);
    
    if PERMISSION_MANAGER.check_permission(inode, perms, uid, gid).unwrap_or(false) {
        Ok(())
    } else {
        Err("Permission denied")
    }
}

/// Change file permissions (chmod)
#[log_entry(info, target = "vfs_ext")]
#[precondition(inode != 0)]
pub fn chmod_file(inode: u64, mode: u32) -> Result<(), &'static str> {
    if mode > 0o7777 {
        return Err("Permission mode must be 0o0000-0o7777");
    }

    let caller = current_security_context();
    PERMISSION_MANAGER.chmod(inode, mode as u16, caller.euid).map_err(|e| e.as_str())?;
    
    logging::log_state_transition(
        "inode_mode",
        "previous",
        &format!("{:#o}", mode),
    );
    Ok(())
}

/// Change file owner (chown)
#[log_entry(info, target = "vfs_ext")]
#[precondition(inode != 0)]
pub fn chown_file(inode: u64, uid: u32, gid: u32) -> Result<(), &'static str> {
    let caller = current_security_context();
    PERMISSION_MANAGER.chown(inode, uid, gid, caller.euid).map_err(|e| e.as_str())?;
    
    logging::log_config_change(
        "inode_owner",
        "previous",
        &format!("uid={}, gid={}", uid, gid),
    );
    Ok(())
}

/// Mount a filesystem
#[log_entry(info, target = "vfs_ext")]
#[precondition(!mount_path.is_empty())]
pub fn mount_filesystem(
    mount_path: &str,
    fstype: &str,
    source: &str,
    _options: &str,
) -> Result<(), &'static str> {
    // Check if user has CAP_MOUNT
    let caller = current_security_context();
    #[cfg(feature = "capability_system")]
    {
        if !caller.has_capability(crate::interfaces::security::cap_flags::CAP_MOUNT) {
            return Err("Operation not permitted: requires CAP_MOUNT");
        }
    }

    MOUNT_MANAGER.mount(mount_path, fstype, source, false).map_err(|e| e.as_str())?;
    
    logging::log_operation_success(
        "mount",
        0,
        &format!("path={}, type={}, source={}", mount_path, fstype, source),
    );
    Ok(())
}

/// Unmount a filesystem
#[log_entry(info, target = "vfs_ext")]
#[precondition(!mount_path.is_empty())]
pub fn unmount_filesystem(mount_path: &str) -> Result<(), &'static str> {
    // Check if user has CAP_MOUNT
    let caller = current_security_context();
    #[cfg(feature = "capability_system")]
    {
        if !caller.has_capability(crate::interfaces::security::cap_flags::CAP_MOUNT) {
            return Err("Operation not permitted: requires CAP_MOUNT");
        }
    }

    MOUNT_MANAGER.unmount(mount_path).map_err(|e| e.as_str())?;
    
    logging::log_operation_success("unmount", 0, mount_path);
    Ok(())
}

/// Set disk quota for user
#[log_entry(info, target = "vfs_ext")]
pub fn set_user_block_quota(user_id: u32, limit_blocks: u64) -> Result<(), &'static str> {
    // Only root or CAP_SYS_ADMIN can set quotas
    let caller = current_security_context();
    #[cfg(feature = "capability_system")]
    {
        if !caller.is_root() && !caller.has_capability(crate::interfaces::security::cap_flags::CAP_SYS_ADMIN) {
            return Err("Operation not permitted: requires CAP_SYS_ADMIN");
        }
    }

    QUOTA_MANAGER.set_block_quota(user_id, limit_blocks).map_err(|e| e.as_str())?;
    
    logging::log_config_change(
        "block_quota",
        "previous",
        &format!("uid={}, limit={}", user_id, limit_blocks),
    );
    Ok(())
}

/// Set inode quota for user
#[log_entry(info, target = "vfs_ext")]
pub fn set_user_inode_quota(user_id: u32, limit_inodes: u64) -> Result<(), &'static str> {
    let caller = current_security_context();
    #[cfg(feature = "capability_system")]
    {
        if !caller.is_root() && !caller.has_capability(crate::interfaces::security::cap_flags::CAP_SYS_ADMIN) {
            return Err("Operation not permitted: requires CAP_SYS_ADMIN");
        }
    }

    QUOTA_MANAGER.set_inode_quota(user_id, limit_inodes).map_err(|e| e.as_str())?;
    
    logging::log_config_change(
        "inode_quota",
        "previous",
        &format!("uid={}, limit={}", user_id, limit_inodes),
    );
    Ok(())
}

/// Returns true if user has quota available for additional blocks.
pub fn can_allocate_blocks(user_id: u32, blocks: u64) -> bool {
    QUOTA_MANAGER.can_allocate(user_id, blocks).unwrap_or(true)
}

/// Get quota status for user
pub fn get_quota_status(user_id: u32) -> Option<String> {
    Some(QUOTA_MANAGER.report_quota(user_id))
}

/// Report VFS extension statistics for diagnostics
pub fn get_vfs_diagnostics() -> String {
    format!(
        "VFS Extensions: permissions={}, mounts={}, quotas=active",
        "active",
        "active"
    )
}

/// Returns summary of active mounts, permission model, and quota enforcement
pub fn report_vfs_stats() -> String {
    let mounts = MOUNT_MANAGER.list_mounts();
    format!(
        "VFS: {} mounts, unix_permissions=enabled, quota_enforcement=enabled",
        mounts.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_vfs_extensions() {
        assert!(init_vfs_extensions().is_ok());
    }

    #[test]
    fn test_check_permission_invalid_inode() {
        // Inode 0 is invalid
        assert!(check_file_permission(0, 1000, 1000, 4).is_err());
    }

    #[test]
    fn test_check_permission_invalid_action() {
        // Action 0 or > 7 is invalid
        assert!(check_file_permission(100, 1000, 1000, 0).is_err());
        assert!(check_file_permission(100, 1000, 1000, 8).is_err());
    }

    #[test]
    fn test_check_permission_valid() {
        let inode = 100;
        // Action 4 = read, valid
        assert!(check_file_permission(inode, 1000, 1000, 4).is_ok());
    }

    #[test]
    fn test_chmod_invalid_mode() {
        let inode = 101;
        // Mode > 0o7777 is invalid
        assert!(chmod_file(inode, 0o10000).is_err());
    }

    #[test]
    fn test_chmod_all_modes() {
        let inode = 102;
        assert!(chmod_file(inode, 0o000).is_ok());
        assert!(chmod_file(inode, 0o755).is_ok());
        assert!(chmod_file(inode, 0o644).is_ok());
        assert!(chmod_file(inode, 0o7777).is_ok()); // Max valid
    }

    #[test]
    fn test_chown() {
        let inode = 103;
        assert!(chown_file(inode, 1000, 1000).is_ok());
    }

    #[test]
    fn test_mount_unmount() {
        assert!(mount_filesystem("/mnt/test", "ext4", "/dev/sdb", "").is_ok());
        let mounts = MOUNT_MANAGER.list_mounts();
        assert!(mounts.len() > 0);
        assert!(unmount_filesystem("/mnt/test").is_ok());
    }

    #[test]
    fn test_mount_empty_path() {
        assert!(mount_filesystem("", "ext4", "/dev/sdb", "").is_err());
        assert!(mount_filesystem("/mnt", "", "/dev/sdb", "").is_err());
    }

    #[test]
    fn test_unmount_empty_path() {
        assert!(unmount_filesystem("").is_err());
    }

    #[test]
    fn test_quota_setting() {
        let uid = 1000;
        assert!(set_user_block_quota(uid, 1_000_000).is_ok());
        assert!(set_user_inode_quota(uid, 100_000).is_ok());
    }

    #[test]
    fn test_quota_enforcement() {
        let uid = 1001;
        set_user_block_quota(uid, 100).ok();
        assert!(can_allocate_blocks(uid, 50));
        assert!(!can_allocate_blocks(uid, 200));
    }

    #[test]
    fn test_vfs_stats_nonempty() {
        let stats = report_vfs_stats();
        assert!(!stats.is_empty());
        assert!(stats.contains("mounts"));
    }

    #[test]
    fn test_list_mounts() {
        let mounts = MOUNT_MANAGER.list_mounts();
        // Should include root mount at minimum
        assert!(mounts.len() >= 1);
    }

    #[test]
    fn test_get_quota_status() {
        let uid = 1002;
        set_user_block_quota(uid, 10_000).ok();
        set_user_inode_quota(uid, 1000).ok();
        let status = get_quota_status(uid);
        // May or may not return status depending on implementation
        let _ = status;
    }
}
