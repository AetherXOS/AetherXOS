use super::*;

pub fn read_user_sockaddr(ptr: usize, len: usize) -> Result<alloc::vec::Vec<u8>, usize> {
    if len == 0 || len > LinuxCompatConfig::MAX_SOCKADDR_LEN {
        return Err(linux_inval());
    }
    with_user_read_bytes(ptr, len, |src| src.to_vec()).map_err(|_| linux_eacces())
}

#[cfg(feature = "posix_net")]
pub fn read_sockaddr_in(
    ptr: usize,
    len: usize,
) -> Result<crate::modules::libnet::PosixSocketAddrV4, usize> {
    if len < core::mem::size_of::<LinuxSockAddrIn>() {
        return Err(linux_inval());
    }

    with_user_read_bytes(ptr, core::mem::size_of::<LinuxSockAddrIn>(), |src| {
        let mut tmp = LinuxSockAddrIn {
            sin_family: 0,
            sin_port: 0,
            sin_addr: [0; 4],
            sin_zero: [0; 8],
        };
        let dst_ptr = &mut tmp as *mut LinuxSockAddrIn as *mut u8;
        let dst = unsafe {
            core::slice::from_raw_parts_mut(dst_ptr, core::mem::size_of::<LinuxSockAddrIn>())
        };
        dst.copy_from_slice(src);

        if i32::from(tmp.sin_family) != crate::modules::posix_consts::net::AF_INET {
            return Err(linux_errno(
                crate::modules::posix_consts::errno::EAFNOSUPPORT,
            ));
        }

        Ok(crate::modules::libnet::PosixSocketAddrV4 {
            addr: tmp.sin_addr,
            port: u16::from_be(tmp.sin_port),
        })
    })
    .map_err(|_| linux_eacces())?
}

pub fn read_user_iovec(ptr: usize, count: usize) -> Result<alloc::vec::Vec<LinuxIoVec>, usize> {
    if count == 0 {
        return Ok(alloc::vec::Vec::new());
    }
    if count > LinuxCompatConfig::MAX_IOV_COUNT {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let mut out = alloc::vec::Vec::with_capacity(count);
    let item_size = core::mem::size_of::<LinuxIoVec>();

    for i in 0..count {
        let item = with_user_read_bytes(ptr + i * item_size, item_size, |src| {
            let mut tmp = LinuxIoVec {
                iov_base: 0,
                iov_len: 0,
            };
            let dst = &mut tmp as *mut LinuxIoVec as *mut u8;
            unsafe {
                core::slice::from_raw_parts_mut(dst, item_size).copy_from_slice(src);
            }
            tmp
        })
        .map_err(|_| linux_eacces())?;
        out.push(item);
    }
    Ok(out)
}

pub fn read_user_c_string(ptr: usize, max_len: usize) -> Result<alloc::string::String, usize> {
    if ptr < USER_SPACE_BOTTOM_INCLUSIVE || ptr >= USER_SPACE_TOP_EXCLUSIVE || max_len == 0 {
        return Err(linux_eacces());
    }

    let mut out = alloc::vec::Vec::new();
    for i in 0..max_len {
        let Some(addr) = ptr.checked_add(i) else {
            return Err(linux_eacces());
        };
        if !user_readable_range_valid(addr, 1) {
            return Err(linux_eacces());
        }
        let b = unsafe { *(addr as *const u8) };
        if b == 0 {
            if out.is_empty() {
                // Allow empty string if it's just a null terminator.
                return Ok(alloc::string::String::new());
            }
            return alloc::string::String::from_utf8(out).map_err(|_| linux_inval());
        }
        out.push(b);
    }

    Err(linux_inval())
}

#[cfg(feature = "posix_net")]
pub fn write_sockaddr_in(
    ptr: usize,
    len_ptr: usize,
    addr: crate::modules::libnet::PosixSocketAddrV4,
) -> usize {
    let want_len = core::mem::size_of::<LinuxSockAddrIn>();

    let given_len = with_user_read_bytes(len_ptr, core::mem::size_of::<u32>(), |src| {
        u32::from_ne_bytes([src[0], src[1], src[2], src[3]]) as usize
    })
    .unwrap_or(0);

    if given_len < want_len {
        return linux_inval();
    }

    let rc = with_user_write_bytes(ptr, want_len, |dst| {
        let sa = LinuxSockAddrIn {
            sin_family: crate::modules::posix_consts::net::AF_INET as u16,
            sin_port: addr.port.to_be(),
            sin_addr: addr.addr,
            sin_zero: [0; 8],
        };
        let sa_ptr = &sa as *const LinuxSockAddrIn as *const u8;
        let sa_bytes = unsafe { core::slice::from_raw_parts(sa_ptr, want_len) };
        dst.copy_from_slice(sa_bytes);
        0usize
    })
    .unwrap_or_else(|_| linux_eacces());

    if rc != 0 {
        return rc;
    }

    with_user_write_bytes(len_ptr, core::mem::size_of::<u32>(), |dst| {
        dst.copy_from_slice(&(want_len as u32).to_ne_bytes());
        0usize
    })
    .unwrap_or_else(|_| linux_eacces())
}

#[cfg(feature = "posix_fs")]
pub fn resolve_linux_at(
    dirfd: Fd,
    pathname_ptr: UserPtr<u8>,
) -> Result<(u32, alloc::string::String, alloc::string::String), usize> {
    let mut path = read_user_c_string(
        pathname_ptr.addr,
        crate::config::KernelConfig::vfs_max_mount_path(),
    )?;

    if path.is_empty() {
        return Err(linux_errno(crate::modules::posix_consts::errno::ENOENT));
    }

    use crate::kernel::syscalls::syscalls_consts::linux;
    let chroot = crate::modules::linux_compat::mount::get_chroot_path();

    // If chroot is set (not "/"), prepend it to the path if it's absolute.
    if path.starts_with('/') {
        if chroot != "/" {
            path = if chroot.ends_with('/') {
                format!("{}{}", chroot, &path[1..])
            } else {
                format!("{}{}", chroot, path)
            };
        }
        return Ok((
            crate::modules::posix::fs::default_fs_id().map_err(|e| linux_errno(e.code()))?,
            alloc::string::String::from("/"),
            path,
        ));
    }

    let fs_id = if dirfd.0 == linux::AT_FDCWD as i32 {
        crate::modules::posix::fs::default_fs_id().map_err(|e| linux_errno(e.code()))?
    } else if dirfd.0 >= 0 {
        match crate::modules::posix::fs::fd_fs_context(dirfd.as_u32()) {
            Ok(id) => id,
            Err(e) => return Err(linux_errno(e.code())),
        }
    } else {
        return Err(linux_errno(crate::modules::posix_consts::errno::EBADF));
    };

    let mut dir_path = if dirfd.0 == linux::AT_FDCWD as i32 {
        crate::modules::posix::fs::getcwd(fs_id).unwrap_or_else(|_| alloc::string::String::from("/"))
    } else {
        crate::modules::posix::fs::fd_path(dirfd.as_u32())
            .unwrap_or_else(|_| alloc::string::String::from("/"))
    };

    // If dir_path doesn't start with chroot, clamp it to chroot root.
    if chroot != "/" && !dir_path.starts_with(&chroot) {
        dir_path = chroot;
    }

    Ok((fs_id, dir_path, path))
}

pub fn read_user_string_vec(
    ptr: usize,
    max_mount_path: usize,
) -> Result<alloc::vec::Vec<alloc::string::String>, usize> {
    if ptr == 0 {
        return Ok(alloc::vec::Vec::new());
    }

    let mut out = alloc::vec::Vec::new();
    let mut off = 0usize;
    loop {
        let word = with_user_read_bytes(ptr + off, core::mem::size_of::<usize>(), |src| {
            let mut tmp = [0u8; core::mem::size_of::<usize>()];
            tmp.copy_from_slice(src);
            usize::from_ne_bytes(tmp)
        })
        .map_err(|_| linux_eacces())?;

        if word == 0 {
            break;
        }

        let s = read_user_c_string(word, max_mount_path)?;
        out.push(s);
        off = off.saturating_add(core::mem::size_of::<usize>());
    }
    Ok(out)
}
