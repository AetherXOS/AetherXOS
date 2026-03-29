use super::*;

pub(crate) fn sys_linux_capget(
    hdr_ptr: UserPtr<LinuxCapUserHeader>,
    data_ptr: UserPtr<LinuxCapUserData>,
) -> usize {
    if hdr_ptr.is_null() {
        return linux_fault();
    }

    let mut hdr = match hdr_ptr.read() {
        Ok(v) => v,
        Err(e) => return e,
    };
    if hdr.version == 0 {
        hdr.version = LINUX_CAP_VERSION_3;
        let _ = hdr_ptr.write(&hdr);
    }

    if !data_ptr.is_null() {
        let zero = LinuxCapUserData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        };
        let _ = data_ptr.write(&zero);
    }
    0
}

pub(crate) fn sys_linux_capset(
    _hdr_ptr: UserPtr<LinuxCapUserHeader>,
    _data_ptr: UserPtr<LinuxCapUserData>,
) -> usize {
    // Capability updates are accepted in this compatibility profile.
    0
}

pub fn sys_linux_rt_sigqueueinfo(pid: usize, sig: usize, _info: UserPtr<u8>) -> usize {
    let _ = _info;
    sys_linux_kill(pid, sig)
}

pub fn sys_linux_sysfs(option: usize, arg1: usize, arg2: usize) -> usize {
    match option {
        // Return canonical "nodev" fs type at index 0.
        1 => {
            if arg1 != 0 {
                return linux_inval();
            }
            if arg2 == 0 {
                return linux_fault();
            }
            let name = b"nodev\0";
            let rc = crate::kernel::syscalls::with_user_write_bytes(arg2, name.len(), |dst| {
                dst.copy_from_slice(name);
                0
            });
            rc.map(|_| 0).unwrap_or_else(|e| e)
        }
        SYSFS_OPTION_2_FILESYSTEM_TYPE_NAME => {
            if arg1 == 0 {
                return linux_fault();
            }
            let name = b"nodev\0";
            let rc = crate::kernel::syscalls::with_user_write_bytes(arg1, name.len(), |dst| {
                dst.copy_from_slice(name);
                0
            });
            rc.map(|_| 0).unwrap_or_else(|e| e)
        }
        SYSFS_OPTION_3_FILESYSTEM_INDEX_BY_NAME => {
            if arg1 == 0 {
                return linux_fault();
            }
            match read_user_c_string(
                arg1,
                crate::modules::linux_compat::config::LinuxCompatConfig::MAX_PATH_LEN,
            ) {
                Ok(name) if name == SYSFS_NODEV_FS_NAME => 0,
                Ok(_) => linux_inval(),
                Err(e) => e,
            }
        }
        _ => linux_inval(),
    }
}

pub fn sys_linux_sysctl(args_ptr: UserPtr<HyperCompatSysctlArgs>) -> usize {
    if args_ptr.is_null() {
        return linux_fault();
    }
    if !crate::config::KernelConfig::should_expose_sysctl_surface() {
        return linux_eperm();
    }

    let args = match args_ptr.read() {
        Ok(v) => v,
        Err(e) => return e,
    };
    let flags = args.flags as usize;
    if flags == 0
        || (flags & !(HC_SYSCTL_FLAG_READ | HC_SYSCTL_FLAG_WRITE | HC_SYSCTL_FLAG_PATH)) != 0
    {
        return linux_inval();
    }

    let key = if args.key_ptr == 0 {
        alloc::string::String::new()
    } else {
        match read_user_c_string(
            args.key_ptr as usize,
            core::cmp::max(args.key_len as usize, 1),
        ) {
            Ok(v) => v,
            Err(e) => return e,
        }
    };

    if (flags & HC_SYSCTL_FLAG_WRITE) != 0 {
        if let Err(e) =
            require_control_plane_access(crate::modules::security::RESOURCE_SECURITY_POLICY)
        {
            return e;
        }
        if args.value_ptr == 0 {
            return linux_fault();
        }
        let value = match read_user_c_string(
            args.value_ptr as usize,
            core::cmp::max(args.value_len as usize, 1),
        ) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let apply_res = if (flags & HC_SYSCTL_FLAG_PATH) != 0 {
            crate::modules::linux_compat::write_compat_config_path(key.as_str(), value.as_str())
        } else {
            crate::modules::linux_compat::apply_compat_config_key(key.as_str(), value.as_str())
        };
        if apply_res.is_err() {
            return linux_inval();
        }
    }

    if (flags & HC_SYSCTL_FLAG_READ) != 0 {
        if args.out_ptr == 0 || args.out_len == 0 {
            return linux_fault();
        }
        let rendered = if (flags & HC_SYSCTL_FLAG_PATH) != 0 {
            crate::modules::linux_compat::read_compat_config_path(key.as_str())
        } else {
            crate::modules::linux_compat::render_compat_config_key(key.as_str())
        };
        let rendered: alloc::string::String = match rendered {
            Ok(v) => v,
            Err(_) => return linux_inval(),
        };
        let bytes = rendered.as_bytes();
        let copy_len = core::cmp::min(bytes.len(), args.out_len as usize);
        let rc = crate::kernel::syscalls::with_user_write_bytes(
            args.out_ptr as usize,
            copy_len,
            |dst| {
                dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
                0usize
            },
        );
        if let Err(e) = rc {
            return e;
        }
        return copy_len;
    }

    0
}
