use super::*;
use alloc::string::ToString;
use core::sync::atomic::Ordering;

pub fn sys_linux_timer_create(
    _clockid: usize,
    _sevp: UserPtr<u8>,
    timerid_ptr: UserPtr<i32>,
) -> usize {
    if timerid_ptr.is_null() {
        return linux_fault();
    }
    let id = NEXT_LINUX_TIMER_ID.fetch_add(1, Ordering::Relaxed);
    LINUX_TIMER_IDS.lock().insert(id);
    timerid_ptr
        .write(&(id as i32))
        .map(|_| 0)
        .unwrap_or_else(|e| e)
}

pub fn sys_linux_timer_delete(timerid: usize) -> usize {
    let removed = LINUX_TIMER_IDS.lock().remove(&(timerid as u32));
    if removed {
        0
    } else {
        linux_inval()
    }
}

pub fn sys_linux_open_by_handle_at(mount_fd: Fd, handle: UserPtr<u8>, flags: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_READ) {
        return e;
    }
    if handle.is_null() {
        return linux_fault();
    }
    if (flags & !OPEN_BY_HANDLE_ALLOWED_FLAGS) != 0 {
        return linux_inval();
    }
    crate::require_posix_fs!((mount_fd, handle, flags) => {
        let mount_fd = mount_fd.as_u32();
        if crate::modules::posix::fs::fd_fs_context(mount_fd).is_err() {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        }
        match crate::modules::posix::fs::dup(mount_fd) {
            Ok(fd) => {
                if (flags & linux::open_flags::O_CLOEXEC) != 0 {
                    crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                        fd,
                        crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                    );
                }
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_memfd_create(name_ptr: UserPtr<u8>, flags: usize) -> usize {
    use crate::kernel::syscalls::syscalls_consts::linux::memfd_flags::{
        MFD_ALLOW_SEALING, MFD_CLOEXEC, MFD_EXEC, MFD_HUGETLB, MFD_NOEXEC_SEAL,
    };

    let known_flags = MFD_CLOEXEC
        | MFD_ALLOW_SEALING
        | MFD_HUGETLB
        | MFD_NOEXEC_SEAL
        | MFD_EXEC
        | linux::MFD_HUGE_MASK;
    if (flags & !known_flags) != 0 {
        return linux_inval();
    }
    if (flags & MFD_EXEC) != 0 && (flags & MFD_NOEXEC_SEAL) != 0 {
        return linux_inval();
    }

    crate::require_posix_fs!((name_ptr, flags) => {
        let raw_name = if name_ptr.is_null() {
            "memfd".to_string()
        } else {
            match read_user_c_string(name_ptr.addr, 255) {
                Ok(s) if !s.is_empty() => s,
                Ok(_) => "memfd".to_string(),
                Err(e) => return e,
            }
        };
        let id = NEXT_MEMFD_ID.fetch_add(1, Ordering::Relaxed);
        let path = alloc::format!("/.memfd-{}-{}", id, raw_name.replace('/', "_"));

        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => {
                if (flags & MFD_CLOEXEC) != 0 {
                    crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                        fd,
                        crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                    );
                } else {
                    crate::modules::linux_compat::fs::io::linux_fd_clear_descriptor_flags(fd);
                }
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_kexec_file_load(
    kernel_fd: Fd,
    initrd_fd: Fd,
    cmdline_len: usize,
    cmdline: UserPtr<u8>,
    flags: usize,
) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_MODULE_LOAD) {
        return e;
    }
    if flags != 0 {
        return linux_inval();
    }
    if cmdline_len > 0 && cmdline.is_null() {
        return linux_fault();
    }
    if cmdline_len > 0 {
        let _ = match read_user_c_string(cmdline.addr, cmdline_len.saturating_add(1)) {
            Ok(v) => v,
            Err(e) => return e,
        };
    }
    KEXEC_STAGED_STATE
        .lock()
        .replace((kernel_fd.as_u32(), initrd_fd.as_u32(), cmdline_len));
    0
}

pub fn sys_linux_bpf(cmd: usize, attr: UserPtr<u8>, size: usize) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_MODULE_LOAD) {
        return e;
    }
    if size == 0 || attr.is_null() {
        return linux_inval();
    }
    match cmd {
        BPF_CMD_MAP_CREATE => {
            let id = NEXT_BPF_MAP_ID.fetch_add(1, Ordering::Relaxed);
            BPF_MAP_IDS.lock().insert(id);
            BPF_MAP_FD_BASE.saturating_add(id as usize)
        }
        _ => linux_inval(),
    }
}

pub fn sys_linux_pkey_mprotect(addr: UserPtr<u8>, len: usize, prot: usize, pkey: usize) -> usize {
    if pkey != 0 {
        return linux_inval();
    }
    sys_linux_mprotect(addr, len, prot)
}

pub fn sys_linux_pkey_alloc(flags: usize, access_rights: usize) -> usize {
    if flags != 0 || access_rights != 0 {
        return linux_inval();
    }
    0
}

pub fn sys_linux_pkey_free(pkey: usize) -> usize {
    if pkey != 0 {
        return linux_inval();
    }
    0
}

pub fn sys_linux_io_pgetevents(
    _ctx_id: usize,
    _min_nr: usize,
    nr: usize,
    events: UserPtr<u8>,
    _timeout: UserPtr<u8>,
    _sig: UserPtr<u8>,
) -> usize {
    if nr == 0 || _min_nr > nr {
        return linux_inval();
    }
    if events.is_null() {
        return linux_fault();
    }
    0
}

pub(crate) fn sys_linux_clone3(
    frame: &mut SyscallFrame,
    clone_args: UserPtr<LinuxCloneArgs>,
    size: usize,
) -> usize {
    if clone_args.is_null() || size < core::mem::size_of::<LinuxCloneArgs>() {
        return linux_inval();
    }
    let args = match clone_args.read() {
        Ok(v) => v,
        Err(e) => return e,
    };

    sys_linux_clone(
        args.flags as usize,
        UserPtr::new(args.stack as usize),
        UserPtr::new(args.parent_tid as usize),
        UserPtr::new(args.child_tid as usize),
        args.tls as usize,
        0,
        frame.rip as usize,
        frame.rflags as usize,
    )
}
