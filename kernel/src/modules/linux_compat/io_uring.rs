use super::*;
use crate::kernel::syscalls::io_uring::{IoUring, GLOBAL_IO_URING_REGISTRY};
use core::sync::atomic::Ordering;
use alloc::sync::Arc;
use spin::Mutex;

use crate::modules::vfs::types::{File, FileStats, IoVec, IoVecMut, PollEvents, SeekFrom};
use crate::modules::posix::fs::BoxedFile;
use core::any::Any;

fn handle_uring_entry(ring: &Arc<IoUring>, entry: &crate::kernel::syscalls::io_uring::IoUringEntry) {
    let res = match entry.opcode {
        0 | 1 => { // READ | WRITE
            let fd = entry.fd;
            let addr = entry.addr;
            let len = entry.len as usize;
            let offset = entry.offset;
            
            let shared_res = crate::modules::posix::fs::get_file_description(fd);
            match shared_res {
                Ok(shared) => {
                    let mut handle = shared.handle.lock();
                    let buf = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, len) };
                    
                    let io_res = if entry.opcode == 0 {
                        handle.read(buf)
                    } else {
                        handle.write(buf)
                    };

                    match io_res {
                        Ok(n) => n as i32,
                        Err(_) => -1,
                    }
                }
                Err(_) => -1,
            }
        }
        2 => { // FSYNC
            let fd = entry.fd;
            let shared_res = crate::modules::posix::fs::get_file_description(fd);
            match shared_res {
                Ok(shared) => {
                    let mut handle = shared.handle.lock();
                    match handle.fsync() {
                        Ok(_) => 0,
                        Err(_) => -1,
                    }
                }
                Err(_) => -1,
            }
        }
        _ => -1,
    };
    
    ring.push_completion(entry.user_data, res);
}

pub struct IoUringFile {
    pub ring: Arc<IoUring>,
}

impl File for IoUringFile {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn poll_events(&self) -> PollEvents {
        let mut events = PollEvents::empty();
        let head = self.ring.cq_head.load(Ordering::Relaxed);
        let tail = self.ring.cq_tail.load(Ordering::Acquire);
        if head != tail {
            events |= PollEvents::IN;
        }
        events
    }
    fn mmap(&self, _offset: u64, _len: usize) -> Result<Arc<Mutex<alloc::vec::Vec<u8>>>, &'static str> {
        Err("mmap for io_uring requires physical page mapping (Phase 4)")
    }
    
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

pub fn sys_linux_io_uring_setup(entries: usize, params_ptr: UserPtr<LinuxIoUringParams>) -> usize {
    if entries == 0 || entries > 4096 {
        return linux_inval();
    }
    
    let mut flags = 0u32;
    if !params_ptr.is_null() {
        if let Ok(params) = params_ptr.read() {
            flags = params.flags;
        }
    }

    let ring = Arc::new(IoUring::new(entries));
    let file = alloc::boxed::Box::new(IoUringFile { ring: ring.clone() });
    
    crate::require_posix_fs!((entries, params_ptr) => {
        let fs_id = *crate::modules::posix::fs::SHM_FS_ID;
        let fd = crate::modules::posix::fs::register_handle(
            fs_id,
            alloc::format!("io_uring:{}", entries),
            Arc::new(Mutex::new(BoxedFile { inner: file })),
            false,
        );
        
        GLOBAL_IO_URING_REGISTRY.lock().insert(fd, ring.clone());
        
        // Handle SQPOLL
        if (flags & 2) != 0 { // IORING_SETUP_SQPOLL
            let ring_worker = ring.clone();
            crate::kernel::task::kthread::spawn_kthread("io_uring_poll", move || {
                loop {
                    if ring_worker.kernel_process(|entry| handle_uring_entry(&ring_worker, entry)) == 0 {
                        crate::hal::HAL::cpu_relax();
                        // Sleep if idle for too long (simplified)
                    }
                }
            });
        }

        if !params_ptr.is_null() {
            let mut params = params_ptr.read().unwrap_or(unsafe { core::mem::zeroed::<LinuxIoUringParams>() });
            params.sq_entries = entries as u32;
            params.cq_entries = (entries * 2) as u32;
            params.features = 1; // FEAT_SINGLE_MMAP
            
            // Define offsets for mmap
            params.sq_off.head = 0;
            params.sq_off.tail = 64;
            params.sq_off.ring_mask = 128;
            params.sq_off.ring_entries = 132;
            params.sq_off.array = 256;
            
            params.cq_off.head = 1024;
            params.cq_off.tail = 1088;
            params.cq_off.cqes = 1280;
            
            let _ = params_ptr.write(&params);
        }
        
        fd as usize
    })
}

pub fn sys_linux_io_uring_enter(
    fd: Fd,
    to_submit: usize,
    min_complete: usize,
    flags: usize,
    _sig: UserPtr<u8>,
) -> usize {
    crate::require_posix_fs!((fd) => {
        let shared = match crate::modules::posix::fs::get_file_description(fd.as_u32()) {
            Ok(f) => f,
            Err(e) => return linux_errno(e.code()),
        };
        
        let handle = shared.handle.lock();
        let ring = if let Some(bf) = handle.as_any().downcast_ref::<BoxedFile>() {
            if let Some(iuf) = bf.inner.as_any().downcast_ref::<IoUringFile>() {
                iuf.ring.clone()
            } else {
                return linux_errno(crate::modules::posix_consts::errno::EBADF);
            }
        } else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };

        if to_submit > 0 {
            ring.kernel_process(|entry| handle_uring_entry(&ring, entry));
        }

        if (flags & 1) != 0 { // IORING_ENTER_GETEVENTS
            while ring.cq_tail.load(Ordering::Acquire).wrapping_sub(ring.cq_head.load(Ordering::Relaxed)) < min_complete as u32 {
                core::hint::spin_loop();
            }
        }

        to_submit
    })
}

pub fn sys_linux_io_uring_register(
    fd: Fd,
    opcode: usize,
    _arg: UserPtr<u8>,
    _nr_args: usize,
) -> usize {
    crate::require_posix_fs!((fd) => {
        let shared = match crate::modules::posix::fs::get_file_description(fd.as_u32()) {
            Ok(f) => f,
            Err(e) => return linux_errno(e.code()),
        };
        
        let handle = shared.handle.lock();
        if let Some(bf) = handle.as_any().downcast_ref::<BoxedFile>() {
            if !bf.inner.as_any().is::<IoUringFile>() {
                return linux_errno(crate::modules::posix_consts::errno::EBADF);
            }
        } else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        }
        
        match opcode {
            0 => 0, // IORING_REGISTER_BUFFERS
            1 => 0, // IORING_REGISTER_FILES
            _ => linux_inval(),
        }
    })
}
