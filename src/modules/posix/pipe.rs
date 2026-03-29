use spin::Mutex;

use crate::modules::posix::PosixErrno;

const PIPE_IO_SPIN_BUDGET: usize = 4096;

use crate::modules::vfs::types::FileStats;
use crate::modules::vfs::File;
use crate::modules::vfs::SeekFrom;
use alloc::sync::Arc;
use core::any::Any;

struct PipeState {
    rb: crate::modules::ipc::RingBuffer,
    readers: u32,
    writers: u32,
}

struct PipeFile {
    state: Arc<Mutex<PipeState>>,
    read_end: bool,
    nonblock: bool,
}

impl File for PipeFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if !self.read_end {
            return Err("not a read end");
        }

        for _ in 0..PIPE_IO_SPIN_BUDGET {
            let state = self.state.lock();
            if let Some(n) = state.rb.receive_internal(buf) {
                return Ok(n);
            }
            if state.writers == 0 {
                return Ok(0);
            }
            if self.nonblock {
                return Err("would block");
            }
            drop(state);
            crate::kernel::rt_preemption::request_forced_reschedule();
        }
        Err("timeout")
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if self.read_end {
            return Err("not a write end");
        }
        if buf.is_empty() {
            return Ok(0);
        }

        for _ in 0..PIPE_IO_SPIN_BUDGET {
            let state = self.state.lock();
            if state.readers == 0 {
                return Err("broken pipe");
            }

            let before = state.rb.stats();
            state.rb.send_internal(buf);
            let after = state.rb.stats();
            if after.send_enqueued > before.send_enqueued {
                return Ok(buf.len());
            }

            if self.nonblock {
                return Err("would block");
            }
            drop(state);
            crate::kernel::rt_preemption::request_forced_reschedule();
        }
        Err("timeout")
    }

    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Err("illegal seek on pipe")
    }

    fn poll_events(&self) -> crate::modules::vfs::types::PollEvents {
        let state = self.state.lock();
        let mut revents = 0u32;
        if self.read_end {
            if state.rb.has_data() {
                revents |= crate::modules::posix_consts::net::POLLIN as u32;
            }
            if state.writers == 0 {
                revents |= crate::modules::posix_consts::net::POLLHUP as u32;
            }
        } else {
            if state.readers != 0 && state.rb.has_space_for(1) {
                revents |= crate::modules::posix_consts::net::POLLOUT as u32;
            }
            if state.readers == 0 {
                revents |= crate::modules::posix_consts::net::POLLERR as u32;
            }
        }
        crate::modules::vfs::types::PollEvents::from_bits_truncate(revents)
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o010666, // S_IFIFO | 0666
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Drop for PipeFile {
    fn drop(&mut self) {
        let mut state = self.state.lock();
        if self.read_end {
            state.readers = state.readers.saturating_sub(1);
        } else {
            state.writers = state.writers.saturating_sub(1);
        }
    }
}

pub fn pipe() -> Result<(u32, u32), PosixErrno> {
    pipe2(false)
}

pub fn pipe2(nonblock: bool) -> Result<(u32, u32), PosixErrno> {
    let state = Arc::new(Mutex::new(PipeState {
        rb: crate::modules::ipc::RingBuffer::new(),
        readers: 1,
        writers: 1,
    }));

    let rf = PipeFile {
        state: state.clone(),
        read_end: true,
        nonblock,
    };
    let wf = PipeFile {
        state,
        read_end: false,
        nonblock,
    };

    let r_fd = crate::modules::posix::fs::register_posix_handle(Arc::new(Mutex::new(rf)))?;
    let w_fd = crate::modules::posix::fs::register_posix_handle(Arc::new(Mutex::new(wf)))?;

    if nonblock {
        let read_flags = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
        let write_flags = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
        let _ = crate::modules::posix::fs::fcntl_set_status_flags(r_fd, read_flags);
        let _ = crate::modules::posix::fs::fcntl_set_status_flags(w_fd, write_flags);
    }

    Ok((r_fd, w_fd))
}

pub fn pipe2_flags(flags: i32) -> Result<(u32, u32), PosixErrno> {
    let nonblock = (flags & crate::modules::posix_consts::net::O_NONBLOCK as i32) != 0;
    pipe2(nonblock)
}

pub fn set_nonblock(fd: u32, enabled: bool) -> Result<(), PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    let mut handle = desc.file.handle.lock();
    if let Some(pipe) = handle.as_any_mut().downcast_mut::<PipeFile>() {
        pipe.nonblock = enabled;
        Ok(())
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}

pub fn close(fd: u32) -> Result<(), PosixErrno> {
    let mut table = crate::modules::posix::fs::FILE_TABLE.lock();
    table.remove(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
    Ok(())
}

pub fn read(fd: u32, dst: &mut [u8]) -> Result<usize, PosixErrno> {
    crate::modules::posix::fs::read(fd, dst)
}

pub fn write(fd: u32, src: &[u8]) -> Result<usize, PosixErrno> {
    crate::modules::posix::fs::write(fd, src)
}
