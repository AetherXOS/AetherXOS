use super::super::*;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

const XATTR_CREATE: usize = 0x1;
const XATTR_REPLACE: usize = 0x2;
const XATTR_MAX_VALUE_SIZE: usize = 64 * 1024;

lazy_static! {
    static ref XATTR_STORE: Mutex<BTreeMap<String, BTreeMap<String, Vec<u8>>>> =
        Mutex::new(BTreeMap::new());
}

fn xattr_key_for_path(dirfd: Fd, path_ptr: UserPtr<u8>) -> Result<String, usize> {
    let (fs_id, dir_path, path) =
        crate::modules::linux_compat::helpers::resolve_linux_at(dirfd, path_ptr)?;
    let resolved = crate::modules::posix::fs::resolve_at_path(fs_id, &dir_path, &path)
        .map_err(|e| linux_errno(e.code()))?;
    Ok(alloc::format!("{}:{}", fs_id, resolved))
}

fn xattr_key_for_fd(fd: Fd) -> Result<String, usize> {
    let fs_id =
        crate::modules::posix::fs::fd_fs_context(fd.as_u32()).map_err(|e| linux_errno(e.code()))?;
    let path =
        crate::modules::posix::fs::fd_path(fd.as_u32()).map_err(|e| linux_errno(e.code()))?;
    Ok(alloc::format!("{}:{}", fs_id, path))
}

fn xattr_read_name(name_ptr: UserPtr<u8>) -> Result<String, usize> {
    let name = read_user_c_string(name_ptr.addr, 256)?;
    if name.is_empty() || !name.contains('.') {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(name)
}

fn xattr_read_value(value_ptr: UserPtr<u8>, size: usize) -> Result<Vec<u8>, usize> {
    if size > XATTR_MAX_VALUE_SIZE {
        return Err(linux_errno(crate::modules::posix_consts::errno::E2BIG));
    }
    let mut out = vec![0u8; size];
    if size > 0 {
        value_ptr.read_bytes(&mut out)?;
    }
    Ok(out)
}

fn xattr_set(key: &str, name: String, value: Vec<u8>, flags: usize) -> usize {
    if (flags & !(XATTR_CREATE | XATTR_REPLACE)) != 0 {
        return linux_inval();
    }
    let mut store = XATTR_STORE.lock();
    let attrs = store.entry(String::from(key)).or_insert_with(BTreeMap::new);
    let exists = attrs.contains_key(&name);
    if (flags & XATTR_CREATE) != 0 && exists {
        return linux_errno(crate::modules::posix_consts::errno::EEXIST);
    }
    if (flags & XATTR_REPLACE) != 0 && !exists {
        return linux_errno(crate::modules::posix_consts::errno::ENOENT);
    }
    attrs.insert(name, value);
    0
}

fn xattr_get(key: &str, name: &str, value: UserPtr<u8>, size: usize) -> usize {
    let data = {
        let store = XATTR_STORE.lock();
        match store.get(key).and_then(|m| m.get(name)) {
            Some(v) => v.clone(),
            None => return linux_errno(crate::modules::posix_consts::errno::ENOENT),
        }
    };
    if size == 0 {
        return data.len();
    }
    if size < data.len() {
        return linux_errno(crate::modules::posix_consts::errno::ERANGE);
    }
    match value.write_bytes(&data) {
        Ok(()) => data.len(),
        Err(e) => e,
    }
}

fn xattr_list(key: &str, list_ptr: UserPtr<u8>, size: usize) -> usize {
    let payload = {
        let store = XATTR_STORE.lock();
        let mut out = Vec::new();
        if let Some(attrs) = store.get(key) {
            for name in attrs.keys() {
                out.extend_from_slice(name.as_bytes());
                out.push(0);
            }
        }
        out
    };
    if size == 0 {
        return payload.len();
    }
    if size < payload.len() {
        return linux_errno(crate::modules::posix_consts::errno::ERANGE);
    }
    match list_ptr.write_bytes(&payload) {
        Ok(()) => payload.len(),
        Err(e) => e,
    }
}

fn xattr_remove(key: &str, name: &str) -> usize {
    let mut store = XATTR_STORE.lock();
    let Some(attrs) = store.get_mut(key) else {
        return linux_errno(crate::modules::posix_consts::errno::ENOENT);
    };
    if attrs.remove(name).is_none() {
        return linux_errno(crate::modules::posix_consts::errno::ENOENT);
    }
    if attrs.is_empty() {
        store.remove(key);
    }
    0
}

pub fn sys_linux_fgetxattr(fd: Fd, name: UserPtr<u8>, value: UserPtr<u8>, size: usize) -> usize {
    crate::require_posix_fs!((fd, name, value, size) => {
        let key = match xattr_key_for_fd(fd) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let name = match xattr_read_name(name) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_get(&key, &name, value, size)
    })
}

pub fn sys_linux_fsetxattr(
    fd: Fd,
    name: UserPtr<u8>,
    value: UserPtr<u8>,
    size: usize,
    flags: usize,
) -> usize {
    crate::require_posix_fs!((fd, name, value, size, flags) => {
        let key = match xattr_key_for_fd(fd) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let name = match xattr_read_name(name) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let value = match xattr_read_value(value, size) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_set(&key, name, value, flags)
    })
}

pub fn sys_linux_flistxattr(fd: Fd, list: UserPtr<u8>, size: usize) -> usize {
    crate::require_posix_fs!((fd, list, size) => {
        let key = match xattr_key_for_fd(fd) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_list(&key, list, size)
    })
}

pub fn sys_linux_fremovexattr(fd: Fd, name: UserPtr<u8>) -> usize {
    crate::require_posix_fs!((fd, name) => {
        let key = match xattr_key_for_fd(fd) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let name = match xattr_read_name(name) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_remove(&key, &name)
    })
}

pub fn sys_linux_getxattr(
    path_ptr: UserPtr<u8>,
    name: UserPtr<u8>,
    value: UserPtr<u8>,
    size: usize,
) -> usize {
    crate::require_posix_fs!((path_ptr, name, value, size) => {
        let key = match xattr_key_for_path(Fd(linux::AT_FDCWD as i32), path_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let name = match xattr_read_name(name) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_get(&key, &name, value, size)
    })
}

pub fn sys_linux_lgetxattr(
    path_ptr: UserPtr<u8>,
    name: UserPtr<u8>,
    value: UserPtr<u8>,
    size: usize,
) -> usize {
    sys_linux_getxattr(path_ptr, name, value, size)
}

pub fn sys_linux_setxattr(
    path_ptr: UserPtr<u8>,
    name: UserPtr<u8>,
    value: UserPtr<u8>,
    size: usize,
    flags: usize,
) -> usize {
    crate::require_posix_fs!((path_ptr, name, value, size, flags) => {
        let key = match xattr_key_for_path(Fd(linux::AT_FDCWD as i32), path_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let name = match xattr_read_name(name) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let value = match xattr_read_value(value, size) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_set(&key, name, value, flags)
    })
}

pub fn sys_linux_lsetxattr(
    path_ptr: UserPtr<u8>,
    name: UserPtr<u8>,
    value: UserPtr<u8>,
    size: usize,
    flags: usize,
) -> usize {
    sys_linux_setxattr(path_ptr, name, value, size, flags)
}

pub fn sys_linux_listxattr(path_ptr: UserPtr<u8>, list: UserPtr<u8>, size: usize) -> usize {
    crate::require_posix_fs!((path_ptr, list, size) => {
        let key = match xattr_key_for_path(Fd(linux::AT_FDCWD as i32), path_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_list(&key, list, size)
    })
}

pub fn sys_linux_llistxattr(path_ptr: UserPtr<u8>, list: UserPtr<u8>, size: usize) -> usize {
    sys_linux_listxattr(path_ptr, list, size)
}

pub fn sys_linux_removexattr(path_ptr: UserPtr<u8>, name: UserPtr<u8>) -> usize {
    crate::require_posix_fs!((path_ptr, name) => {
        let key = match xattr_key_for_path(Fd(linux::AT_FDCWD as i32), path_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let name = match xattr_read_name(name) {
            Ok(v) => v,
            Err(e) => return e,
        };
        xattr_remove(&key, &name)
    })
}

pub fn sys_linux_lremovexattr(path_ptr: UserPtr<u8>, name: UserPtr<u8>) -> usize {
    sys_linux_removexattr(path_ptr, name)
}
