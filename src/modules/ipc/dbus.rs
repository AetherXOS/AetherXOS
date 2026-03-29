use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

const DBUS_TOPIC_QUEUE_LIMIT: usize = 256;

static DBUS_SUBSCRIBE_CALLS: AtomicU64 = AtomicU64::new(0);
static DBUS_PUBLISH_CALLS: AtomicU64 = AtomicU64::new(0);
static DBUS_CONSUME_CALLS: AtomicU64 = AtomicU64::new(0);
static DBUS_PUBLISH_DROPS: AtomicU64 = AtomicU64::new(0);
static DBUS_CONSUME_HITS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct DbusStats {
    pub subscribe_calls: u64,
    pub publish_calls: u64,
    pub consume_calls: u64,
    pub publish_drops: u64,
    pub consume_hits: u64,
    pub topics: usize,
}

use crate::interfaces::task::TaskId;

lazy_static! {
    static ref DBUS_QUEUES: Mutex<BTreeMap<String, VecDeque<Vec<u8>>>> =
        Mutex::new(BTreeMap::new());
    static ref DBUS_SUBSCRIBERS: Mutex<BTreeMap<String, Vec<TaskId>>> = Mutex::new(BTreeMap::new());
    static ref DBUS_WAITERS: Mutex<BTreeMap<String, WaitQueue>> = Mutex::new(BTreeMap::new());
}

pub fn dbus_subscribe(topic: &str) -> Result<(), &'static str> {
    DBUS_SUBSCRIBE_CALLS.fetch_add(1, Ordering::Relaxed);
    if topic.is_empty() {
        return Err("topic empty");
    }

    let tid = unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| TaskId(cpu.current_task.load(Ordering::Relaxed)))
            .unwrap_or(TaskId(0))
    };

    {
        let mut queues = DBUS_QUEUES.lock();
        queues.entry(topic.into()).or_insert_with(VecDeque::new);
    }
    {
        let mut waiters = DBUS_WAITERS.lock();
        waiters.entry(topic.into()).or_insert_with(WaitQueue::new);
    }

    let mut subs = DBUS_SUBSCRIBERS.lock();
    let entry = subs.entry(topic.into()).or_insert_with(Vec::new);
    if !entry.contains(&tid) {
        entry.push(tid);
    }
    Ok(())
}

pub fn dbus_publish(topic: &str, payload: &[u8]) -> Result<usize, &'static str> {
    DBUS_PUBLISH_CALLS.fetch_add(1, Ordering::Relaxed);
    if topic.is_empty() {
        return Err("topic empty");
    }

    {
        let mut queues = DBUS_QUEUES.lock();
        let queue = queues.entry(topic.into()).or_insert_with(VecDeque::new);
        if queue.len() >= DBUS_TOPIC_QUEUE_LIMIT {
            DBUS_PUBLISH_DROPS.fetch_add(1, Ordering::Relaxed);
            return Err("topic queue full");
        }
        queue.push_back(payload.to_vec());
    }

    // Wake one waiting consumer
    let mut waiters_map = DBUS_WAITERS.lock();
    if let Some(wq) = waiters_map.get_mut(topic) {
        if let Some(tid) = wq.wake_one() {
            crate::kernel::task::wake_task(tid);
        }
    }

    Ok(payload.len())
}

pub fn dbus_consume(topic: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    DBUS_CONSUME_CALLS.fetch_add(1, Ordering::Relaxed);

    loop {
        {
            let mut queues = DBUS_QUEUES.lock();
            let queue = queues.get_mut(topic).ok_or("topic not found")?;
            if let Some(frame) = queue.pop_front() {
                let copied = core::cmp::min(frame.len(), out.len());
                out[..copied].copy_from_slice(&frame[..copied]);
                DBUS_CONSUME_HITS.fetch_add(1, Ordering::Relaxed);
                return Ok(copied);
            }
        }

        // Nothing available: Block wait
        let wq = {
            let mut waiters_map = DBUS_WAITERS.lock();
            waiters_map
                .entry(topic.into())
                .or_insert_with(WaitQueue::new)
                .clone()
        };

        crate::kernel::task::suspend_current_task(&wq);
    }
}

pub fn dbus_stats() -> DbusStats {
    DbusStats {
        subscribe_calls: DBUS_SUBSCRIBE_CALLS.load(Ordering::Relaxed),
        publish_calls: DBUS_PUBLISH_CALLS.load(Ordering::Relaxed),
        consume_calls: DBUS_CONSUME_CALLS.load(Ordering::Relaxed),
        publish_drops: DBUS_PUBLISH_DROPS.load(Ordering::Relaxed),
        consume_hits: DBUS_CONSUME_HITS.load(Ordering::Relaxed),
        topics: DBUS_QUEUES.lock().len(),
    }
}
