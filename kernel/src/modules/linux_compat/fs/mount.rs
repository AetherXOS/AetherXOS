use super::super::*;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicI32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

#[cfg(feature = "vfs")]
lazy_static! {
    static ref MOUNT_CONTEXTS: Mutex<BTreeMap<i32, MountContext>> = Mutex::new(BTreeMap::new());
    static ref MOUNT_FDS: Mutex<BTreeMap<i32, MountFdState>> = Mutex::new(BTreeMap::new());
    static ref CHROOT_PATH: Mutex<alloc::string::String> =
        Mutex::new(alloc::string::String::from("/"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MountCtxKind {
    RamFs,
}

#[derive(Clone, Copy, Debug)]
struct MountContext {
    kind: MountCtxKind,
    readonly: bool,
}

#[derive(Clone, Copy, Debug)]
enum MountFdState {
    Detached(MountContext),
    Attached { mount_id: usize },
}

static NEXT_MOUNT_CTX_FD: AtomicI32 = AtomicI32::new(linux::MOUNT_CTX_FD_BASE as i32);
static NEXT_MOUNT_FD: AtomicI32 = AtomicI32::new(linux::MOUNT_FD_BASE as i32);
const MS_RDONLY: usize = 1;
const MS_REMOUNT: usize = 32;
const MS_BIND: usize = 4096;
const FSCONFIG_SET_FLAG: usize = 0;
const FSCONFIG_SET_STRING: usize = 1;
const FSCONFIG_SET_BINARY: usize = 2;
const FSCONFIG_SET_PATH: usize = 3;
const FSCONFIG_SET_PATH_EMPTY: usize = 4;
const FSCONFIG_SET_FD: usize = 5;
const FSCONFIG_CMD_CREATE: usize = 6;
const FSCONFIG_CMD_RECONFIGURE: usize = 7;

pub fn sys_linux_chroot(pathname: UserPtr<u8>) -> usize {
    crate::require_posix_fs!((pathname) => {
        let path = match read_user_c_string(pathname.addr, 256) { Ok(p) => p, Err(e) => return e };
        if path.is_empty() { return linux_inval(); }

        // In a true production system, we'd verify path existence and directory status.
        // For now, we update our process-global root path.
        let mut root = CHROOT_PATH.lock();
        *root = path;
        0
    })
}

pub fn sys_linux_pivot_root(new_root: UserPtr<u8>, old_put: UserPtr<u8>) -> usize {
    crate::require_posix_fs!((new_root, old_put) => {
        let _new = match read_user_c_string(new_root.addr, 256) { Ok(p) => p, Err(e) => return e };
        let _old = match read_user_c_string(old_put.addr, 256) { Ok(p) => p, Err(e) => return e };

        // pivot_root is complex and usually requires MS_MOVE type logic.
        // For production-grade shim, we accept it if paths are valid.
        let mut root = CHROOT_PATH.lock();
        *root = _new;
        0
    })
}

pub fn get_chroot_path() -> alloc::string::String {
    CHROOT_PATH.lock().clone()
}

pub fn linux_path_is_readonly(path: &str) -> bool {
    crate::kernel::vfs_control::mount_readonly_by_path(path.as_bytes()).unwrap_or(false)
}

pub fn linux_fd_is_readonly(fd: Fd) -> bool {
    let path = match crate::modules::posix::fs::fd_path(fd.as_u32()) {
        Ok(p) => p,
        Err(_) => return false,
    };
    linux_path_is_readonly(&path)
}

pub fn sys_linux_fsopen(fsname: UserPtr<u8>, flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    let name = match read_user_c_string(fsname.addr, 64) {
        Ok(n) => n,
        Err(e) => return e,
    };
    let kind = match name.as_str() {
        "ramfs" | "tmpfs" => MountCtxKind::RamFs,
        _ => return linux_inval(),
    };
    let fd = NEXT_MOUNT_CTX_FD.fetch_add(1, Ordering::Relaxed);
    MOUNT_CONTEXTS.lock().insert(
        fd,
        MountContext {
            kind,
            readonly: false,
        },
    );
    fd as usize
}

pub fn sys_linux_fsmount(fsfd: Fd, _flags: usize, _attr_flags: usize) -> usize {
    let ctx = match MOUNT_CONTEXTS.lock().get(&fsfd.as_i32()) {
        Some(v) => *v,
        None => return linux_errno(crate::modules::posix_consts::errno::EBADF),
    };
    let fd = NEXT_MOUNT_FD.fetch_add(1, Ordering::Relaxed);
    MOUNT_FDS.lock().insert(fd, MountFdState::Detached(ctx));
    fd as usize
}

pub fn sys_linux_mount_setattr(
    dfd: Fd,
    pathname: UserPtr<u8>,
    _flags: usize,
    attr: UserPtr<LinuxMountAttr>,
    size: usize,
) -> usize {
    if size < core::mem::size_of::<LinuxMountAttr>() {
        return linux_inval();
    }
    let a = match attr.read() {
        Ok(v) => v,
        Err(e) => return e,
    };

    let (fs_id, dir_path, path) = resolve_at!(dfd, pathname);
    let abs_path = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) {
        Ok(p) => p,
        Err(e) => return linux_errno(e.code()),
    };

    if let Some(mount_id) = crate::kernel::vfs_control::mount_id_by_path(abs_path.as_bytes()) {
        if (a.attr_set & linux::mountfd::MOUNT_ATTR_RDONLY) != 0 {
            let _ = crate::kernel::vfs_control::set_mount_readonly(mount_id, true);
        } else if (a.attr_clr & linux::mountfd::MOUNT_ATTR_RDONLY) != 0 {
            let _ = crate::kernel::vfs_control::set_mount_readonly(mount_id, false);
        }
        return 0;
    }
    linux_errno(crate::modules::posix_consts::errno::ENOENT)
}

pub fn sys_linux_move_mount(
    from_dfd: Fd,
    _from_pathname: UserPtr<u8>,
    to_dfd: Fd,
    to_pathname: UserPtr<u8>,
    _flags: usize,
) -> usize {
    let (to_fs, to_dir, to_rel) = resolve_at!(to_dfd, to_pathname);
    let to_abs = match crate::modules::posix::fs::resolve_at_path(to_fs, &to_dir, &to_rel) {
        Ok(p) => p,
        Err(e) => return linux_errno(e.code()),
    };

    let state = match MOUNT_FDS.lock().get(&from_dfd.as_i32()) {
        Some(v) => *v,
        None => return linux_errno(crate::modules::posix_consts::errno::EBADF),
    };
    match state {
        MountFdState::Detached(ctx) => {
            let mount_id = match ctx.kind {
                MountCtxKind::RamFs => {
                    match crate::kernel::vfs_control::mount_ramfs(to_abs.as_bytes()) {
                        Ok(id) => id,
                        Err(_) => return linux_inval(),
                    }
                }
            };
            let _ = crate::kernel::vfs_control::set_mount_readonly(mount_id, ctx.readonly);
            MOUNT_FDS
                .lock()
                .insert(from_dfd.as_i32(), MountFdState::Attached { mount_id });
            0
        }
        MountFdState::Attached { mount_id } => {
            match crate::kernel::vfs_control::relocate_mount(mount_id, to_abs.as_bytes()) {
                Ok(()) => 0,
                Err(_) => linux_inval(),
            }
        }
    }
}

pub fn sys_linux_open_tree(dfd: Fd, pathname: UserPtr<u8>, _flags: usize) -> usize {
    let (fs_id, dir_path, path) = resolve_at!(dfd, pathname);
    let abs_path = match crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path) {
        Ok(p) => p,
        Err(e) => return linux_errno(e.code()),
    };

    if let Some(mount_id) = crate::kernel::vfs_control::mount_id_by_path(abs_path.as_bytes()) {
        let fd = NEXT_MOUNT_FD.fetch_add(1, Ordering::Relaxed);
        MOUNT_FDS
            .lock()
            .insert(fd, MountFdState::Attached { mount_id });
        return fd as usize;
    }
    linux_errno(crate::modules::posix_consts::errno::ENOENT)
}

pub fn sys_linux_fspick(_dfd: Fd, pathname: UserPtr<u8>, _flags: usize) -> usize {
    sys_linux_fsopen(pathname, 0) // Basic pick redirection
}

pub fn sys_linux_mount(
    source: UserPtr<u8>,
    target: UserPtr<u8>,
    _fstype: UserPtr<u8>,
    flags: usize,
    _data: UserPtr<u8>,
) -> usize {
    let target_path = match read_user_c_string(target.addr, 256) {
        Ok(p) => p,
        Err(e) => return e,
    };
    if (flags & MS_REMOUNT) != 0 {
        if let Some(id) = crate::kernel::vfs_control::mount_id_by_path(target_path.as_bytes()) {
            let _ = crate::kernel::vfs_control::set_mount_readonly(id, (flags & MS_RDONLY) != 0);
            return 0;
        }
        return linux_errno(crate::modules::posix_consts::errno::ENOENT);
    }

    if (flags & MS_BIND) != 0 {
        let _ = source;
        linux_inval()
    } else {
        match crate::kernel::vfs_control::mount_ramfs(target_path.as_bytes()) {
            Ok(id) => {
                let _ =
                    crate::kernel::vfs_control::set_mount_readonly(id, (flags & MS_RDONLY) != 0);
                0
            }
            Err(_) => linux_inval(),
        }
    }
}

pub fn sys_linux_umount2(target: UserPtr<u8>, _flags: usize) -> usize {
    let path = match read_user_c_string(target.addr, 256) {
        Ok(p) => p,
        Err(e) => return e,
    };
    match crate::kernel::vfs_control::unmount_by_path(path.as_bytes()) {
        Ok(_) => 0,
        Err(_) => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}

pub fn sys_linux_fsconfig_apply(
    fd: Fd,
    cmd: usize,
    key: UserPtr<u8>,
    _value: UserPtr<u8>,
    _aux: usize,
) -> usize {
    #[cfg(not(feature = "vfs"))]
    {
        let _ = (fd, cmd, key, _value, _aux);
        return linux_inval();
    }
    #[cfg(feature = "vfs")]
    {
        let mut table = MOUNT_CONTEXTS.lock();
        let Some(ctx) = table.get_mut(&fd.as_i32()) else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };

        match cmd {
            FSCONFIG_SET_FLAG => {
                if key.is_null() {
                    return linux_fault();
                }
                let k = match read_user_c_string(key.addr, 64) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                match k.as_str() {
                    "ro" | "rdonly" => ctx.readonly = true,
                    "rw" => ctx.readonly = false,
                    _ => return linux_inval(),
                }
                0
            }
            FSCONFIG_SET_STRING
            | FSCONFIG_SET_BINARY
            | FSCONFIG_SET_PATH
            | FSCONFIG_SET_PATH_EMPTY
            | FSCONFIG_SET_FD
            | FSCONFIG_CMD_CREATE
            | FSCONFIG_CMD_RECONFIGURE => 0,
            _ => linux_inval(),
        }
    }
}
