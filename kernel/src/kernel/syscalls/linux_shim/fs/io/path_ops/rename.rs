#[cfg(feature = "posix_fs")]
use alloc::string::String;
use crate::kernel::syscalls::linux_errno;
use crate::kernel::syscalls::linux_shim::fs::support::resolve_path_at;

pub(crate) fn sys_linux_renameat(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        let (old_fs_id, old_resolved) = match resolve_path_at(olddirfd, oldpath_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let (new_fs_id, new_resolved) = match resolve_path_at(newdirfd, newpath_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if old_fs_id != new_fs_id {
            return linux_errno(crate::modules::posix_consts::errno::EXDEV);
        }
        match crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &new_resolved) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (olddirfd, oldpath_ptr, newdirfd, newpath_ptr);
        linux_errno(crate::modules::posix_consts::errno::ENOENT)
    }
}

pub(crate) fn sys_linux_renameat2(
    olddirfd: isize,
    oldpath_ptr: usize,
    newdirfd: isize,
    newpath_ptr: usize,
    flags: usize,
) -> usize {
    const RENAME_NOREPLACE: usize = 1;
    const RENAME_EXCHANGE: usize = 2;
    const RENAME_WHITEOUT: usize = 4;
    let allowed_flags = RENAME_NOREPLACE | RENAME_EXCHANGE | RENAME_WHITEOUT;

    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & RENAME_NOREPLACE) != 0 && (flags & RENAME_EXCHANGE) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    // Minimal compatibility: accept classic rename behavior when flags==0.
    if flags == 0 {
        return sys_linux_renameat(olddirfd, oldpath_ptr, newdirfd, newpath_ptr);
    }

    #[cfg(feature = "posix_fs")]
    {
        if flags == RENAME_NOREPLACE {
            let (old_fs_id, old_resolved) = match resolve_path_at(olddirfd, oldpath_ptr) {
                Ok(v) => v,
                Err(err) => return err,
            };
            let (new_fs_id, new_resolved) = match resolve_path_at(newdirfd, newpath_ptr) {
                Ok(v) => v,
                Err(err) => return err,
            };
            if old_fs_id != new_fs_id {
                return linux_errno(crate::modules::posix_consts::errno::EXDEV);
            }

            match crate::modules::posix::fs::access(new_fs_id, &new_resolved) {
                Ok(true) => return linux_errno(crate::modules::posix_consts::errno::EEXIST),
                Ok(false) => {}
                Err(err) => return linux_errno(err.code()),
            }

            return match crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &new_resolved)
            {
                Ok(()) => 0,
                Err(err) => linux_errno(err.code()),
            };
        }

        if flags == RENAME_EXCHANGE {
            let (old_fs_id, old_resolved) = match resolve_path_at(olddirfd, oldpath_ptr) {
                Ok(v) => v,
                Err(err) => return err,
            };
            let (new_fs_id, new_resolved) = match resolve_path_at(newdirfd, newpath_ptr) {
                Ok(v) => v,
                Err(err) => return err,
            };
            if old_fs_id != new_fs_id {
                return linux_errno(crate::modules::posix_consts::errno::EXDEV);
            }

            let old_exists = match crate::modules::posix::fs::access(old_fs_id, &old_resolved) {
                Ok(exists) => exists,
                Err(err) => return linux_errno(err.code()),
            };
            let new_exists = match crate::modules::posix::fs::access(new_fs_id, &new_resolved) {
                Ok(exists) => exists,
                Err(err) => return linux_errno(err.code()),
            };
            if !old_exists || !new_exists {
                return linux_errno(crate::modules::posix_consts::errno::ENOENT);
            }

            // Best-effort exchange using a temporary sibling path.
            let mut tmp_path: Option<String> = None;
            for idx in 0..16u8 {
                let mut candidate = new_resolved.clone();
                candidate.push_str(".hc_swap_tmp_");
                let digit = if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + (idx - 10)) as char
                };
                candidate.push(digit);
                match crate::modules::posix::fs::access(new_fs_id, &candidate) {
                    Ok(false) => {
                        tmp_path = Some(candidate);
                        break;
                    }
                    Ok(true) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }

            let Some(tmp_resolved) = tmp_path else {
                return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
            };

            if let Err(err) = crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &tmp_resolved)
            {
                return linux_errno(err.code());
            }

            if let Err(err) = crate::modules::posix::fs::rename(new_fs_id, &new_resolved, &old_resolved)
            {
                let _ = crate::modules::posix::fs::rename(old_fs_id, &tmp_resolved, &old_resolved);
                return linux_errno(err.code());
            }

            if let Err(err) = crate::modules::posix::fs::rename(old_fs_id, &tmp_resolved, &new_resolved)
            {
                let _ = crate::modules::posix::fs::rename(new_fs_id, &old_resolved, &new_resolved);
                let _ = crate::modules::posix::fs::rename(old_fs_id, &tmp_resolved, &old_resolved);
                return linux_errno(err.code());
            }

            return 0;
        }

        if flags == RENAME_WHITEOUT {
            let (old_fs_id, old_resolved) = match resolve_path_at(olddirfd, oldpath_ptr) {
                Ok(v) => v,
                Err(err) => return err,
            };
            let (new_fs_id, new_resolved) = match resolve_path_at(newdirfd, newpath_ptr) {
                Ok(v) => v,
                Err(err) => return err,
            };
            if old_fs_id != new_fs_id {
                return linux_errno(crate::modules::posix_consts::errno::EXDEV);
            }

            if let Err(err) = crate::modules::posix::fs::rename(old_fs_id, &old_resolved, &new_resolved)
            {
                return linux_errno(err.code());
            }

            // Best-effort whiteout marker: recreate source path as hidden placeholder.
            // This keeps lower-layer style lookups blocked in simplified overlay paths.
            if let Err(err) = crate::modules::posix::fs::open(old_fs_id, &old_resolved, true) {
                return linux_errno(err.code());
            }
            let _ = crate::modules::posix::fs::chmod(old_fs_id, &old_resolved, 0o000);
            return 0;
        }
    }

    // Any combination not explicitly modeled above remains invalid for compatibility safety.
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}
