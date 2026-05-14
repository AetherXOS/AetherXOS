use super::*;
use crate::modules::vfs::types::{File, PollEvents};
use core::any::Any;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

pub struct EpollInstance {
    pub interests: Mutex<BTreeMap<u32, EpollEvent>>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EpollEvent {
    pub events: u32,
    pub data: u64,
}

impl File for EpollInstance {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn poll_events(&self) -> PollEvents {
        // Epoll is readable if any of its interests are ready
        let interests = self.interests.lock();
        for (&fd, event) in interests.iter() {
            let target_events = PollEvents::from_bits_truncate(event.events);
            if let Ok(revents) = crate::modules::posix::io::poll_one_vfs(fd, target_events) {
                if !revents.is_empty() {
                    return PollEvents::IN;
                }
            }
        }
        PollEvents::empty()
    }
    
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

pub fn epoll_create(_size: i32) -> Result<u32, PosixErrno> {
    let instance = alloc::boxed::Box::new(EpollInstance {
        interests: Mutex::new(BTreeMap::new()),
    });
    
    let fs_id = *crate::modules::posix::fs::SHM_FS_ID;
    let fd = crate::modules::posix::fs::register_handle(
        fs_id,
        alloc::string::String::from("epoll"),
        Arc::new(Mutex::new(crate::modules::posix::fs::BoxedFile { inner: instance })),
        false,
    );
    Ok(fd)
}

pub fn epoll_ctl(epfd: u32, op: i32, fd: u32, event: &EpollEvent) -> Result<(), PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&epfd).ok_or(PosixErrno::BadFileDescriptor)?;
    let handle = desc.file.handle.lock();
    
    let bf = handle.as_any().downcast_ref::<crate::modules::posix::fs::BoxedFile>()
        .ok_or(PosixErrno::BadFileDescriptor)?;
    let epoll = bf.inner.as_any().downcast_ref::<EpollInstance>()
        .ok_or(PosixErrno::BadFileDescriptor)?;
    
    let mut interests = epoll.interests.lock();
    match op {
        1 => { // EPOLL_CTL_ADD
            if interests.contains_key(&fd) {
                return Err(PosixErrno::AlreadyExists);
            }
            interests.insert(fd, *event);
        }
        2 => { // EPOLL_CTL_DEL
            interests.remove(&fd).ok_or(PosixErrno::NoEntry)?;
        }
        3 => { // EPOLL_CTL_MOD
            let target = interests.get_mut(&fd).ok_or(PosixErrno::NoEntry)?;
            *target = *event;
        }
        _ => return Err(PosixErrno::Invalid),
    }
    Ok(())
}

pub fn epoll_wait(epfd: u32, events: &mut [EpollEvent], timeout_ms: i32) -> Result<usize, PosixErrno> {
    let table = crate::modules::posix::fs::FILE_TABLE.lock();
    let desc = table.get(&epfd).ok_or(PosixErrno::BadFileDescriptor)?;
    let handle = desc.file.handle.lock();
    
    let bf = handle.as_any().downcast_ref::<crate::modules::posix::fs::BoxedFile>()
        .ok_or(PosixErrno::BadFileDescriptor)?;
    let epoll = bf.inner.as_any().downcast_ref::<EpollInstance>()
        .ok_or(PosixErrno::BadFileDescriptor)?;

    let start_ticks = crate::kernel::watchdog::global_tick();
    let _hz = 1000; // Assume 1ms per tick for simplicity or check KernelConfig
    let timeout_ticks = if timeout_ms > 0 { timeout_ms as u64  } else { 0 };

    loop {
        let mut count = 0;
        let mut aggregator = crate::kernel::sync::WaitAggregator::new();
        
        {
            let interests = epoll.interests.lock();
            for (&fd, interest) in interests.iter() {
                let target_events = PollEvents::from_bits_truncate(interest.events);
                
                // Get the file and check for events
                if let Some(desc) = table.get(&fd) {
                    let handle = desc.file.handle.lock();
                    let revents = handle.poll_events();
                    
                    if !(revents & target_events).is_empty() && count < events.len() {
                        events[count] = EpollEvent {
                            events: (revents & target_events).bits(),
                            data: interest.data,
                        };
                        count += 1;
                    }
                    
                    // If no events and we are going to sleep, add to aggregator
                    if count == 0 {
                        if let Some(q) = handle.wait_queue() {
                            aggregator.add(q);
                        }
                    }
                }
            }
        }

        if count > 0 || timeout_ms == 0 {
            return Ok(count);
        }

        if timeout_ms > 0 && crate::kernel::watchdog::global_tick().wrapping_sub(start_ticks) >= timeout_ticks {
            return Ok(0);
        }

        // Sleep until woken by any of the watched files
        aggregator.wait();
    }
}
