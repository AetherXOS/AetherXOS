
use crate::modules::vfs::types::{FileStats, VfsTimespec};
use crate::modules::vfs::File;
use crate::modules::vfs::SeekFrom;
use alloc::sync::Arc;
use core::any::Any;

use crate::kernel::sync::ring_buffer::RingBuffer;
use spin::Mutex;
use crate::modules::posix::PosixErrno;

struct PipeState {
    rb: RingBuffer<u8>,
    readers: u32,
    writers: u32,
}

struct SharedPipe {
    state: Mutex<PipeState>,
    read_waiters: crate::kernel::sync::WaitQueue,
    write_waiters: crate::kernel::sync::WaitQueue,
}

struct PipeFile {
    shared: Arc<SharedPipe>,
    read_end: bool,
    nonblock: bool,
}

impl File for PipeFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if !self.read_end {
            return Err("not a read end");
        }

        loop {
            let state = self.shared.state.lock();
            let count = state.rb.pop_slice(buf);
            
            if count > 0 {
                // Wake up writers that might be waiting for space
                self.shared.write_waiters.wake_all();
                return Ok(count);
            }

            if self.nonblock || state.writers == 0 {
                return Ok(0); // EOF if no writers left
            }

            // Block until data is available
            drop(state);
            crate::kernel::task::suspend_current_task(&self.shared.read_waiters);
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        if self.read_end {
            return Err("not a write end");
        }

        loop {
            let state = self.shared.state.lock();
            if state.readers == 0 {
                return Err("broken pipe");
            }

            let written = state.rb.push_slice(buf);
            if written > 0 {
                // Wake up readers that might be waiting for data
                self.shared.read_waiters.wake_all();
                return Ok(written);
            }

            if self.nonblock {
                return Err("resource temporarily unavailable");
            }

            // Block until space is available
            drop(state);
            crate::kernel::task::suspend_current_task(&self.shared.write_waiters);
        }
    }

    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, &'static str> {
        Err("illegal seek on pipe")
    }

    fn poll_events(&self) -> crate::modules::vfs::types::PollEvents {
        let state = self.shared.state.lock();
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
            size: self.shared.state.lock().rb.len() as u64,
            mode: 0o10666, // S_IFIFO | 0666
            uid: 0,
            gid: 0,
            nlink: 1,
            atime: VfsTimespec::default(),
            mtime: VfsTimespec::default(),
            ctime: VfsTimespec::default(),
            btime: VfsTimespec::default(),
            blksize: 4096,
            blocks: 0,
            ino: 0,
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
        let mut state = self.shared.state.lock();
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
    let shared = Arc::new(SharedPipe {
        state: Mutex::new(PipeState {
            rb: RingBuffer::new(65536), // Linux default 64KB
            readers: 1,
            writers: 1,
        }),
        read_waiters: crate::kernel::sync::WaitQueue::new(),
        write_waiters: crate::kernel::sync::WaitQueue::new(),
    });

    let rf = PipeFile {
        shared: shared.clone(),
        read_end: true,
        nonblock,
    };
    let wf = PipeFile {
        shared,
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
