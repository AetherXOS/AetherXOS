use super::*;

pub fn sys_linux_lseek(fd: Fd, offset: i64, whence: usize) -> usize {
    crate::require_posix_fs!((fd, offset, whence) => {
        let w = match whence {
            linux::seek::SEEK_SET => crate::modules::posix::fs::SeekWhence::Set,
            linux::seek::SEEK_CUR => crate::modules::posix::fs::SeekWhence::Cur,
            linux::seek::SEEK_END => crate::modules::posix::fs::SeekWhence::End,
            _ => return linux_inval(),
        };
        match crate::modules::posix::fs::lseek(fd.as_u32(), offset, w) {
            Ok(p) => p as usize,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_pread64(fd: Fd, buf: UserPtr<u8>, count: usize, offset: usize) -> usize {
    crate::require_posix_fs!((fd, buf, count, offset) => {
        match buf.write_bytes_with(count, |dst| {
            match crate::modules::posix::fs::pread(fd.as_u32(), dst, offset as u64) {
                Ok(n) => n,
                Err(e) => linux_errno(e.code()),
            }
        }) {
            Ok(n) => n,
            Err(e) => e,
        }
    })
}

pub fn sys_linux_pwrite64(fd: Fd, buf: UserPtr<u8>, count: usize, offset: usize) -> usize {
    crate::require_posix_fs!((fd, buf, count, offset) => {
        if super::mount::linux_fd_is_readonly(fd) {
            return linux_errno(crate::modules::posix_consts::errno::EROFS);
        }
        match buf.read_bytes_with(count, |src| {
            match crate::modules::posix::fs::pwrite(fd.as_u32(), src, offset as u64) {
                Ok(n) => n,
                Err(e) => linux_errno(e.code()),
            }
        }) {
            Ok(n) => n,
            Err(e) => e,
        }
    })
}

pub fn sys_linux_readv(fd: Fd, iov_ptr: UserPtr<LinuxIoVec>, iovcnt: usize) -> usize {
    if iovcnt > LinuxCompatConfig::MAX_IOV_COUNT {
        return linux_inval();
    }
    let mut total = 0;
    for i in 0..iovcnt {
        let iov = match iov_ptr.add(i).read() {
            Ok(v) => v,
            Err(e) => return e,
        };
        let n = sys_linux_read(fd, UserPtr::new(iov.iov_base as usize), iov.iov_len as usize);
        if (n as isize) < 0 {
            return if total > 0 { total } else { n };
        }
        total += n;
        if n < iov.iov_len as usize {
            break;
        }
    }
    total
}

pub fn sys_linux_writev(fd: Fd, iov_ptr: UserPtr<LinuxIoVec>, iovcnt: usize) -> usize {
    if iovcnt > LinuxCompatConfig::MAX_IOV_COUNT {
        return linux_inval();
    }
    let mut total = 0;
    for i in 0..iovcnt {
        let iov = match iov_ptr.add(i).read() {
            Ok(v) => v,
            Err(e) => return e,
        };
        let n = sys_linux_write(fd, UserPtr::new(iov.iov_base as usize), iov.iov_len as usize);
        if (n as isize) < 0 {
            return if total > 0 { total } else { n };
        }
        total += n;
        if n < iov.iov_len as usize {
            break;
        }
    }
    total
}

pub fn sys_linux_sendfile(out_fd: Fd, in_fd: Fd, _offset_ptr: UserPtr<i64>, count: usize) -> usize {
    crate::require_posix_fs!((out_fd, in_fd, _offset_ptr, count) => {
        if count == 0 {
            return 0;
        }
        let mut buf = alloc::vec![0u8; count.min(SENDFILE_CHUNK_LIMIT)];
        let mut total = 0;
        let mut rem = count;
        while rem > 0 {
            let chunk = rem.min(buf.len());
            let nread = match crate::modules::posix::fs::read(in_fd.as_u32(), &mut buf[..chunk]) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    return if total > 0 {
                        total
                    } else {
                        linux_errno(e.code())
                    };
                }
            };
            let nwrite = match crate::modules::posix::fs::write(out_fd.as_u32(), &buf[..nread]) {
                Ok(n) => n,
                Err(e) => {
                    return if total > 0 {
                        total
                    } else {
                        linux_errno(e.code())
                    };
                }
            };
            total += nwrite;
            rem -= nwrite;
            if nwrite < nread {
                break;
            }
        }
        total
    })
}

pub fn sys_linux_copy_file_range(
    fd_in: Fd,
    off_in: UserPtr<i64>,
    fd_out: Fd,
    _off_out: UserPtr<i64>,
    len: usize,
    _flags: usize,
) -> usize {
    sys_linux_sendfile(fd_out, fd_in, off_in, len)
}

pub fn sys_linux_splice(
    fd_in: Fd,
    off_in: UserPtr<i64>,
    fd_out: Fd,
    off_out: UserPtr<i64>,
    len: usize,
    _flags: usize,
) -> usize {
    crate::require_posix_pipe!((fd_in, off_in, fd_out, off_out, len, _flags) => {
        // Advanced zero-copy pipeline semantics not yet modeled.
        // Provide semantic fallback: regular sendfile behavior since splice allows
        // moving between file descriptors like sendfile does.
        sys_linux_sendfile(fd_out, fd_in, off_in, len)
    })
}

pub fn sys_linux_tee(fd_in: Fd, fd_out: Fd, len: usize, _flags: usize) -> usize {
    crate::require_posix_pipe!((fd_in, fd_out, len, _flags) => {
        // Full non-consuming tee semantics require pipe-buffer duplication support.
        // Until that low-level support lands, provide a practical compatibility fallback
        // by forwarding through splice/sendfile semantics.
        sys_linux_splice(
            fd_in,
            UserPtr::new(0),
            fd_out,
            UserPtr::new(0),
            len,
            _flags,
        )
    })
}

pub fn sys_linux_vmsplice(fd: Fd, iov: UserPtr<LinuxIoVec>, nr_segs: usize, _flags: usize) -> usize {
    crate::require_posix_pipe!((fd, iov, nr_segs, _flags) => {
        // Vmsplice pushes or pulls user buffers to/from a pipe. Fallback to writev/readv
        // maps to regular memory copies instead of page gifting.
        sys_linux_writev(fd, iov, nr_segs)
    })
}

pub fn sys_linux_fsync(fd: Fd) -> usize {
    crate::require_posix_fs!((fd) => {
        match crate::modules::posix::fs::fsync(fd.as_u32()) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_ftruncate(fd: Fd, length: usize) -> usize {
    crate::require_posix_fs!((fd, length) => {
        if super::mount::linux_fd_is_readonly(fd) {
            return linux_errno(crate::modules::posix_consts::errno::EROFS);
        }
        match crate::modules::posix::fs::ftruncate(fd.as_u32(), length) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}
