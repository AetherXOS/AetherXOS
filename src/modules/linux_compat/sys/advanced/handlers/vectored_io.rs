use super::*;

const COPY_FILE_RANGE_CHUNK_BYTES: usize = 4096;
const OFFSET_USE_FILE_POSITION: usize = usize::MAX;
const MAX_IOVEC_ENTRY_BYTES: usize = usize::MAX / 2;
const RWF_HIPRI: usize = 0x0000_0001;
const RWF_DSYNC: usize = 0x0000_0002;
const RWF_SYNC: usize = 0x0000_0004;
const RWF_NOWAIT: usize = 0x0000_0008;
const RWF_APPEND: usize = 0x0000_0010;
const RWF_ALLOWED_PREADV2: usize = RWF_HIPRI | RWF_DSYNC | RWF_SYNC | RWF_NOWAIT;
const RWF_ALLOWED_PWRITEV2: usize = RWF_ALLOWED_PREADV2 | RWF_APPEND;

fn read_i64_from_user(ptr: UserPtr<i64>) -> Result<i64, usize> {
    if ptr.is_null() {
        return Err(linux_fault());
    }
    ptr.read()
}

fn write_i64_to_user(ptr: UserPtr<i64>, value: i64) -> Result<(), usize> {
    if ptr.is_null() {
        return Err(linux_fault());
    }
    ptr.write(&value)
}

pub fn sys_linux_copy_file_range(
    fd_in: Fd,
    off_in_ptr: UserPtr<i64>,
    fd_out: Fd,
    off_out_ptr: UserPtr<i64>,
    len: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    if len == 0 {
        return 0;
    }

    crate::require_posix_fs!((fd_in, off_in_ptr, fd_out, off_out_ptr, len, flags) => {
        let in_fd = fd_in.as_u32();
        let out_fd = fd_out.as_u32();

        let in_off_explicit = !off_in_ptr.is_null();
        let out_off_explicit = !off_out_ptr.is_null();
        let mut in_off = if in_off_explicit {
            match read_i64_from_user(off_in_ptr) {
                Ok(v) => v,
                Err(e) => return e,
            }
        } else {
            match crate::modules::posix::fs::lseek(in_fd, 0, crate::modules::posix::fs::SeekWhence::Cur) {
                Ok(v) => v as i64,
                Err(e) => return linux_errno(e.code()),
            }
        };
        let mut out_off = if out_off_explicit {
            match read_i64_from_user(off_out_ptr) {
                Ok(v) => v,
                Err(e) => return e,
            }
        } else {
            match crate::modules::posix::fs::lseek(out_fd, 0, crate::modules::posix::fs::SeekWhence::Cur) {
                Ok(v) => v as i64,
                Err(e) => return linux_errno(e.code()),
            }
        };

        let mut copied = 0usize;
        let mut buf = [0u8; COPY_FILE_RANGE_CHUNK_BYTES];
        while copied < len {
            let to_read = core::cmp::min(buf.len(), len - copied);
            let nread = match crate::modules::posix::fs::pread(in_fd, &mut buf[..to_read], in_off as u64) {
                Ok(n) => n,
                Err(e) => return linux_errno(e.code()),
            };
            if nread == 0 {
                break;
            }
            let nwritten = match crate::modules::posix::fs::pwrite(out_fd, &buf[..nread], out_off as u64) {
                Ok(n) => n,
                Err(e) => return linux_errno(e.code()),
            };
            copied = copied.saturating_add(nwritten);
            in_off = in_off.saturating_add(nread as i64);
            out_off = out_off.saturating_add(nwritten as i64);
            if nwritten < nread {
                break;
            }
        }

        if in_off_explicit {
            if let Err(e) = write_i64_to_user(off_in_ptr, in_off) {
                return e;
            }
        } else {
            let _ = crate::modules::posix::fs::lseek(
                in_fd,
                in_off,
                crate::modules::posix::fs::SeekWhence::Set,
            );
        }
        if out_off_explicit {
            if let Err(e) = write_i64_to_user(off_out_ptr, out_off) {
                return e;
            }
        } else {
            let _ = crate::modules::posix::fs::lseek(
                out_fd,
                out_off,
                crate::modules::posix::fs::SeekWhence::Set,
            );
        }
        copied
    })
}

pub fn sys_linux_preadv2(
    fd: Fd,
    iov: UserPtr<LinuxIoVec>,
    iovcnt: usize,
    offset: usize,
    flags: usize,
) -> usize {
    if iovcnt == 0 {
        return 0;
    }
    if iovcnt > linux::IOV_MAX {
        return linux_inval();
    }
    if (flags & !RWF_ALLOWED_PREADV2) != 0 {
        return linux_inval();
    }

    crate::require_posix_fs!((fd, iov, iovcnt, offset, flags) => {
        let file_fd = fd.as_u32();
        let mut total = 0usize;
        let use_file_pos = offset == OFFSET_USE_FILE_POSITION;
        let mut off = if use_file_pos {
            match crate::modules::posix::fs::lseek(file_fd, 0, crate::modules::posix::fs::SeekWhence::Cur) {
                Ok(v) => v as i64,
                Err(e) => return linux_errno(e.code()),
            }
        } else {
            offset as i64
        };
        for idx in 0..iovcnt {
            let ent = match iov.add(idx).read() {
                Ok(v) => v,
                Err(e) => return e,
            };
            if ent.iov_len == 0 {
                continue;
            }
            let req_len = core::cmp::min(ent.iov_len as usize, MAX_IOVEC_ENTRY_BYTES);
            let wrote = with_user_write_bytes(ent.iov_base as usize, req_len, |dst| {
                match if use_file_pos {
                    crate::modules::posix::fs::read(file_fd, dst)
                } else {
                    crate::modules::posix::fs::pread(file_fd, dst, off as u64)
                } {
                    Ok(n) => n as isize,
                    Err(e) => -(e.code() as isize),
                }
            })
            .unwrap_or(-(crate::modules::posix_consts::errno::EFAULT as isize));
            if wrote < 0 {
                if total > 0 {
                    return total;
                }
                return linux_errno((-wrote) as i32);
            }
            let n = wrote as usize;
            total = total.saturating_add(n);
            if !use_file_pos {
                off = off.saturating_add(n as i64);
            }
            if n < req_len {
                break;
            }
        }
        total
    })
}

pub fn sys_linux_preadv(
    fd: Fd,
    iov: UserPtr<LinuxIoVec>,
    iovcnt: usize,
    pos_l: usize,
    pos_h: usize,
) -> usize {
    let off = ((pos_h as u64) << 32) | (pos_l as u64);
    sys_linux_preadv2(fd, iov, iovcnt, off as usize, 0)
}

pub fn sys_linux_pwritev2(
    fd: Fd,
    iov: UserPtr<LinuxIoVec>,
    iovcnt: usize,
    offset: usize,
    flags: usize,
) -> usize {
    if iovcnt == 0 {
        return 0;
    }
    if iovcnt > linux::IOV_MAX {
        return linux_inval();
    }
    if (flags & !RWF_ALLOWED_PWRITEV2) != 0 {
        return linux_inval();
    }

    crate::require_posix_fs!((fd, iov, iovcnt, offset, flags) => {
        let file_fd = fd.as_u32();
        let mut total = 0usize;
        let use_file_pos = offset == OFFSET_USE_FILE_POSITION;
        let mut off = if use_file_pos {
            match crate::modules::posix::fs::lseek(file_fd, 0, crate::modules::posix::fs::SeekWhence::Cur) {
                Ok(v) => v as i64,
                Err(e) => return linux_errno(e.code()),
            }
        } else {
            offset as i64
        };
        for idx in 0..iovcnt {
            let ent = match iov.add(idx).read() {
                Ok(v) => v,
                Err(e) => return e,
            };
            if ent.iov_len == 0 {
                continue;
            }
            let req_len = core::cmp::min(ent.iov_len as usize, MAX_IOVEC_ENTRY_BYTES);
            let wrote = with_user_read_bytes(ent.iov_base as usize, req_len, |src| {
                match if use_file_pos {
                    crate::modules::posix::fs::write(file_fd, src)
                } else {
                    crate::modules::posix::fs::pwrite(file_fd, src, off as u64)
                } {
                    Ok(n) => n as isize,
                    Err(e) => -(e.code() as isize),
                }
            })
            .unwrap_or(-(crate::modules::posix_consts::errno::EFAULT as isize));
            if wrote < 0 {
                if total > 0 {
                    return total;
                }
                return linux_errno((-wrote) as i32);
            }
            let n = wrote as usize;
            total = total.saturating_add(n);
            if !use_file_pos {
                off = off.saturating_add(n as i64);
            }
            if n < req_len {
                break;
            }
        }
        total
    })
}

pub fn sys_linux_pwritev(
    fd: Fd,
    iov: UserPtr<LinuxIoVec>,
    iovcnt: usize,
    pos_l: usize,
    pos_h: usize,
) -> usize {
    let off = ((pos_h as u64) << 32) | (pos_l as u64);
    sys_linux_pwritev2(fd, iov, iovcnt, off as usize, 0)
}
