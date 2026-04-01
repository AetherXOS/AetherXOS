use super::*;
use core::sync::atomic::Ordering;

pub fn sys_linux_vhangup() -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_PROCESS_PTRACE)
    {
        return e;
    }
    // No virtual tty ownership model yet; treat as successful compat action.
    0
}

pub fn sys_linux_acct(filename: UserPtr<u8>) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_READ) {
        return e;
    }
    if filename.is_null() {
        return 0;
    }
    crate::require_posix_fs!((filename) => {
        let path = match read_user_c_string(filename.addr, crate::modules::linux_compat::config::LinuxCompatConfig::MAX_PATH_LEN) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if path.is_empty() {
            return linux_inval();
        }
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::resolve_at_path(fs_id, "/", &path) {
            Ok(_) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_reboot(magic1: usize, magic2: usize, cmd: usize, arg: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_POWER_REBOOT) {
        return e;
    }
    if magic1 != REBOOT_MAGIC1 {
        return linux_inval();
    }
    if !matches!(magic2, REBOOT_MAGIC2_A | REBOOT_MAGIC2_B | REBOOT_MAGIC2_C) {
        return linux_inval();
    }
    if !matches!(
        cmd,
        REBOOT_CMD_RESTART | REBOOT_CMD_HALT | REBOOT_CMD_POWER_OFF | REBOOT_CMD_RESTART2
    ) {
        return linux_inval();
    }
    REBOOT_LAST_CMD.store(cmd as u32, Ordering::Relaxed);
    let _ = arg;
    0
}

pub fn sys_linux_iopl(level: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_SECURITY_POLICY)
    {
        return e;
    }
    if level > IOPL_MAX_LEVEL {
        return linux_inval();
    }
    IOPL_LEVEL.store(level as u32, Ordering::Relaxed);
    0
}

pub fn sys_linux_ioperm(from: usize, num: usize, turn_on: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_SECURITY_POLICY)
    {
        return e;
    }
    if turn_on > 1 || num == 0 {
        return linux_inval();
    }
    let Some(end) = from.checked_add(num) else {
        return linux_inval();
    };
    let mut table = IOPERM_ENABLED_RANGES.lock();
    let range = (from, end);
    if turn_on == 0 {
        table.remove(&range);
    } else {
        table.insert(range);
    }
    0
}

pub fn sys_linux_create_module(name: UserPtr<u8>, size: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_MODULE_LOAD) {
        return e;
    }
    if name.is_null() || size == 0 {
        return linux_inval();
    }
    let name = match read_user_c_string(name.addr, 255) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if name.is_empty() {
        return linux_inval();
    }
    let id = NEXT_LEGACY_MODULE_ID.fetch_add(1, Ordering::Relaxed) as usize;
    LEGACY_MODULES.lock().insert(name, id);
    id
}

pub fn sys_linux_init_module(
    module_image: UserPtr<u8>,
    len: usize,
    param_values: UserPtr<u8>,
) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_MODULE_LOAD) {
        return e;
    }
    if module_image.is_null() || len == 0 {
        return linux_inval();
    }
    let _ = param_values;
    0
}

pub fn sys_linux_delete_module(name: UserPtr<u8>, flags: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_MODULE_UNLOAD) {
        return e;
    }
    if name.is_null() {
        return linux_fault();
    }
    let name = match read_user_c_string(name.addr, 255) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let _ = flags;
    if LEGACY_MODULES.lock().remove(&name).is_some() {
        0
    } else {
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

pub fn sys_linux_security(op: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_SECURITY_POLICY)
    {
        return e;
    }
    let _ = (arg1, arg2, arg3);
    // Minimal LSM query surface for compat callers.
    if op == 0 {
        0
    } else {
        linux_inval()
    }
}
