use super::PosixErrno;
use crate::modules::vfs::File;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

lazy_static::lazy_static! {
    static ref MQ_TABLE: Mutex<BTreeMap<String, Arc<Mutex<MessageQueue>>>> = Mutex::new(BTreeMap::new());
}

struct MessageQueue {
    messages: alloc::collections::BinaryHeap<MqMessage>,
    max_msgs: usize,
    max_msgsize: usize,
    /// Tasks waiting for messages (recv).
    wait_recv: crate::kernel::sync::WaitQueue,
    /// Tasks waiting for space (send).
    wait_send: crate::kernel::sync::WaitQueue,
}

#[derive(Debug, Eq, PartialEq)]
struct MqMessage {
    priority: u32,
    data: Vec<u8>,
    // Counter to maintain FIFO for same priority
    seq: u64,
}

impl Ord for MqMessage {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq)) // Lower seq = earlier = higher priority
    }
}

impl PartialOrd for MqMessage {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct MqFile {
    pub name: String,
    queue: Arc<Mutex<MessageQueue>>,
    pub nonblock: bool,
    next_seq: Arc<core::sync::atomic::AtomicU64>,
}

impl File for MqFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        loop {
            let mut q = self.queue.lock();
            if let Some(msg) = q.messages.pop() {
                let len = core::cmp::min(buf.len(), msg.data.len());
                buf[..len].copy_from_slice(&msg.data[..len]);

                // Wake one sender since there's now space
                if let Some(t) = q.wait_send.wake_one() {
                    crate::kernel::task::wake_task(t);
                }
                return Ok(len);
            } else {
                if self.nonblock {
                    return Err("empty");
                }
                return Err("would block");
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let prio = 0; // Default priority, can be extended via ioctl/syscall params
        loop {
            let mut q = self.queue.lock();
            if q.messages.len() < q.max_msgs {
                if buf.len() > q.max_msgsize {
                    return Err("message too large");
                }
                let seq = self
                    .next_seq
                    .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                q.messages.push(MqMessage {
                    priority: prio,
                    data: buf.to_vec(),
                    seq,
                });

                // Wake one receiver
                if let Some(t) = q.wait_recv.wake_one() {
                    crate::kernel::task::wake_task(t);
                }
                return Ok(buf.len());
            } else {
                if self.nonblock {
                    return Err("full");
                }
                return Err("would block");
            }
        }
    }

    fn poll_events(&self) -> crate::modules::vfs::PollEvents {
        let q = self.queue.lock();
        let mut ev = crate::modules::vfs::PollEvents::empty();
        if !q.messages.is_empty() {
            ev |= crate::modules::vfs::PollEvents::IN;
        }
        if q.messages.len() < q.max_msgs {
            ev |= crate::modules::vfs::PollEvents::OUT;
        }
        ev
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

pub fn mq_open(
    name: &str,
    oflag: i32,
    max_msgs: usize,
    max_msgsize: usize,
) -> Result<u32, PosixErrno> {
    if !name.starts_with('/') {
        return Err(PosixErrno::Invalid);
    }

    let mut table = MQ_TABLE.lock();
    let queue = if let Some(q) = table.get(name) {
        if (oflag & 0x40) != 0 && (oflag & 0x80) != 0 {
            // O_CREAT | O_EXCL
            return Err(PosixErrno::AlreadyExists);
        }
        q.clone()
    } else {
        if (oflag & 0x40) == 0 {
            // O_CREAT
            return Err(PosixErrno::NoEntry);
        }
        let q = Arc::new(Mutex::new(MessageQueue {
            messages: alloc::collections::BinaryHeap::new(),
            max_msgs: if max_msgs == 0 { 10 } else { max_msgs },
            max_msgsize: if max_msgsize == 0 { 8192 } else { max_msgsize },
            wait_recv: crate::kernel::sync::WaitQueue::new(),
            wait_send: crate::kernel::sync::WaitQueue::new(),
        }));
        table.insert(String::from(name), q.clone());
        q
    };

    let nonblock = (oflag & 0x800) != 0;
    let file = MqFile {
        name: String::from(name),
        queue,
        nonblock,
        next_seq: Arc::new(core::sync::atomic::AtomicU64::new(0)),
    };

    let fd = crate::modules::posix::fs::register_handle(
        0,
        alloc::format!("mq:{}", name),
        Arc::new(Mutex::new(file)),
        true,
    );
    Ok(fd)
}

pub fn mq_unlink(name: &str) -> Result<(), PosixErrno> {
    if MQ_TABLE.lock().remove(name).is_some() {
        Ok(())
    } else {
        Err(PosixErrno::NoEntry)
    }
}
