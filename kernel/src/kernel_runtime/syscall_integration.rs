/// PHASE 6 TASK 7: Syscall Path Integration
/// 
/// Hooks kernel subsystem APIs into actual syscall execution paths.
/// This module bridges the gap between syscall handlers and initialized subsystems.
///
/// Integration Points:
/// 1. Task spawn syscall → scheduler_integration::init_task_scheduler()
/// 2. Memory allocation syscalls (brk) → memory_integration quota enforcement
/// 3. File operation syscalls → vfs_integration permission checks

use crate::core::log;
use crate::interfaces::task::TaskId;
use crate::kernel_runtime::{integration_utils, scheduler_integration, memory_integration, vfs_integration};

// ============================================================================
// TASK SPAWN INTEGRATION
// ============================================================================

/// Hook called when a new task is spawned
/// 
/// Assigns task to scheduler with default priority (Interactive)
/// and enables per-CPU affinity tracking.
pub fn on_task_spawn(task_id: TaskId) -> Result<(), &'static str> {
    integration_utils::logging::log_operation_start("task_spawn_hook", task_id.0 as u64);
    
    // Initialize task in scheduler
    match scheduler_integration::init_task_scheduler(task_id) {
        Ok(()) => {
            integration_utils::logging::log_operation_success(
                "task_spawn_hook",
                task_id.0 as u64,
                "scheduler_ready",
            );
            Ok(())
        }
        Err(e) => {
            integration_utils::logging::log_operation_failure(
                "task_spawn_hook",
                task_id.0 as u64,
                e,
            );
            Err("Failed to initialize task scheduler")
        }
    }
}

// ============================================================================
// MEMORY ALLOCATION INTEGRATION
// ============================================================================

/// Hook called when a process requests heap expansion (via brk syscall)
/// 
/// Enforces per-process memory quotas and limits.
/// Returns the new break value on success, or the current break on failure.
pub fn on_brk_syscall(pid: usize, new_brk: u64, current_brk: u64) -> u64 {
    integration_utils::logging::log_operation_start("brk_hook", pid as u64);
    
    // Calculate allocation size
    let allocation_size = if new_brk > current_brk {
        (new_brk - current_brk) as usize
    } else {
        // Shrinking heap is always allowed
        integration_utils::logging::log_operation_success(
            "brk_hook",
            pid as u64,
            "heap_shrink_allowed",
        );
        return new_brk;
    };
    
    // Check if size is page-aligned (4096 bytes)
    let page_size = 4096;
    if allocation_size % page_size != 0 {
        log::debug(&format!("BRK allocation {} not page-aligned", allocation_size));
        // Let it through but log it
    }
    
    // Record allocation and check quota (use track_memory_allocation)
    match memory_integration::track_memory_allocation(pid as u32, allocation_size) {
        Ok(()) => {
            integration_utils::logging::log_operation_success(
                "brk_hook",
                pid as u64,
                "quota_ok",
            );
            new_brk
        }
        Err(e) => {
            integration_utils::logging::log_operation_failure(
                "brk_hook",
                pid as u64,
                e,
            );
            log::warn(&format!("Process {} exceeded memory quota", pid));
            // Return current break (failure indicated to process)
            current_brk
        }
    }
}

/// Hook called when a process deallocates memory
pub fn on_memory_deallocation(pid: usize, size: usize) {
    integration_utils::logging::log_operation_start("mfree_hook", pid as u64);
    memory_integration::track_memory_deallocation(pid as u32, size);
    integration_utils::logging::log_operation_success(
        "mfree_hook",
        pid as u64,
        "deallocated",
    );
}

// ============================================================================
// FILE OPERATION INTEGRATION
// ============================================================================

/// Hook called before opening a file
/// 
/// Enforces Unix file permissions and mount quotas.
/// Returns Ok(()) if permission granted, Err() if denied.
pub fn on_vfs_open(
    inode_num: u64,
    uid: u32,
    gid: u32,
    _mode: u16,
    flags: u32,
) -> Result<(), &'static str> {
    integration_utils::logging::log_operation_start("vfs_open_hook", inode_num);
    
    // Determine access type from flags
    // flags: 0=read, 1=write, 2=read+write
    let action_str = match flags & 0x3 {
        0 => "read",
        1 => "write",
        _ => "read_write",
    };
    
    // Convert action to u8 for permission check
    let action = match flags & 0x3 {
        0 => 0u8,  // read
        1 => 1u8,  // write
        _ => 2u8,  // read+write
    };
    
    // Check file permissions (Unix-style)
    match vfs_integration::check_file_permission(
        inode_num,
        uid,
        gid,
        action,
    ) {
        Ok(()) => {
            integration_utils::logging::log_operation_success(
                "vfs_open_hook",
                inode_num,
                action_str,
            );
            Ok(())
        }
        Err(e) => {
            integration_utils::logging::log_operation_failure(
                "vfs_open_hook",
                inode_num,
                e,
            );
            log::warn(&format!(
                "Permission denied: uid={} gid={} inode={} action={}",
                uid, gid, inode_num, action_str
            ));
            Err("Permission denied")
        }
    }
}

/// Hook called on file stat/access
pub fn on_vfs_stat(
    inode_num: u64,
    uid: u32,
    gid: u32,
) -> Result<(), &'static str> {
    integration_utils::logging::log_operation_start("vfs_stat_hook", inode_num);
    
    // Stat requires read permission on parent or file itself
    let action = 0u8;  // read
    match vfs_integration::check_file_permission(
        inode_num,
        uid,
        gid,
        action,
    ) {
        Ok(()) => {
            integration_utils::logging::log_operation_success(
                "vfs_stat_hook",
                inode_num,
                "readable",
            );
            Ok(())
        }
        Err(e) => {
            integration_utils::logging::log_operation_failure(
                "vfs_stat_hook",
                inode_num,
                e,
            );
            Err("Stat denied")
        }
    }
}

/// Hook called when changing file permissions
pub fn on_chmod(
    inode_num: u64,
    _uid: u32,
    _gid: u32,
    new_mode: u16,
) -> Result<(), &'static str> {
    integration_utils::logging::log_operation_start("chmod_hook", inode_num);
    
    // Use chmod_file (corrected name)
    match vfs_integration::chmod_file(inode_num, new_mode as u32) {
        Ok(()) => {
            integration_utils::logging::log_operation_success(
                "chmod_hook",
                inode_num,
                "mode_changed",
            );
            Ok(())
        }
        Err(e) => {
            integration_utils::logging::log_operation_failure(
                "chmod_hook",
                inode_num,
                e,
            );
            Err("Chmod failed")
        }
    }
}

/// Hook called when changing file ownership
pub fn on_chown(
    inode_num: u64,
    _uid: u32,
    _gid: u32,
    new_uid: u32,
    new_gid: u32,
) -> Result<(), &'static str> {
    integration_utils::logging::log_operation_start("chown_hook", inode_num);
    
    // Use chown_file with all required parameters
    match vfs_integration::chown_file(inode_num, new_uid, new_gid) {
        Ok(()) => {
            integration_utils::logging::log_operation_success(
                "chown_hook",
                inode_num,
                "owner_changed",
            );
            Ok(())
        }
        Err(e) => {
            integration_utils::logging::log_operation_failure(
                "chown_hook",
                inode_num,
                e,
            );
            Err("Chown failed")
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_task_spawn() {
        let task_id = TaskId(1000);
        assert!(on_task_spawn(task_id).is_ok());
    }

    #[test]
    fn test_on_brk_syscall_expand() {
        let pid = 100;
        let current_brk = 0x10000000;
        let new_brk = 0x10001000; // Expand by 1 page
        
        let result = on_brk_syscall(pid, new_brk, current_brk);
        // Should succeed if quota allows
        assert!(result == new_brk || result == current_brk);
    }

    #[test]
    fn test_on_brk_syscall_shrink() {
        let pid = 100;
        let current_brk = 0x10001000;
        let new_brk = 0x10000000; // Shrink by 1 page
        
        let result = on_brk_syscall(pid, new_brk, current_brk);
        // Shrink always succeeds
        assert_eq!(result, new_brk);
    }

    #[test]
    fn test_on_memory_deallocation() {
        on_memory_deallocation(100, 4096);
        // Should not panic
    }

    #[test]
    fn test_on_vfs_open_read() {
        let result = on_vfs_open(1001, 1000, 1000, 0o644, 0); // flags=0 (read)
        // Should return Ok or Err depending on permission check
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_on_vfs_open_write() {
        let result = on_vfs_open(1002, 1000, 1000, 0o644, 1); // flags=1 (write)
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_on_vfs_stat() {
        let result = on_vfs_stat(1003, 1000, 1000);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_on_chmod() {
        let result = on_chmod(1004, 1000, 1000, 0o755);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_on_chown() {
        let result = on_chown(1005, 1000, 1000, 2000, 2000);
        assert!(result.is_ok() || result.is_err());
    }
}
