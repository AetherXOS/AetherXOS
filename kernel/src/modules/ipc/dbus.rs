use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use spin::Mutex;
use super::common::{bounded_push_bytes, suspend_on, wake_one_task};

const DBUS_TOPIC_QUEUE_LIMIT: usize = 256;

declare_counter_u64!(DBUS_SUBSCRIBE_CALLS);
declare_counter_u64!(DBUS_PUBLISH_CALLS);
declare_counter_u64!(DBUS_CONSUME_CALLS);
declare_counter_u64!(DBUS_PUBLISH_DROPS);
declare_counter_u64!(DBUS_CONSUME_HITS);
declare_counter_u64!(DBUS_SERVICE_REGISTRATIONS);
declare_counter_u64!(DBUS_SERVICE_HEARTBEATS);

#[derive(Debug, Clone, Copy)]
pub struct DbusStats {
    pub subscribe_calls: u64,
    pub publish_calls: u64,
    pub consume_calls: u64,
    pub publish_drops: u64,
    pub consume_hits: u64,
    pub topics: usize,
    pub session_services: usize,
    pub service_registrations: u64,
    pub service_heartbeats: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SessionServiceState {
    Starting = 0,
    Ready = 1,
    Degraded = 2,
}

impl_enum_u8_default_conversions!(SessionServiceState { Starting, Ready, Degraded }, default = Starting);

#[derive(Debug, Clone)]
struct SessionServiceEntry {
    state: SessionServiceState,
    auto_restart: bool,
    restart_count: u32,
    last_heartbeat_tick: u64,
}

#[derive(Debug, Clone)]
pub struct SessionServiceSnapshot {
    pub name: String,
    pub state: SessionServiceState,
    pub auto_restart: bool,
    pub restart_count: u32,
    pub last_heartbeat_tick: u64,
}

use crate::interfaces::task::TaskId;

lazy_static! {
    static ref DBUS_QUEUES: Mutex<BTreeMap<String, VecDeque<Vec<u8>>>> =
        Mutex::new(BTreeMap::new());
    static ref DBUS_SUBSCRIBERS: Mutex<BTreeMap<String, Vec<TaskId>>> = Mutex::new(BTreeMap::new());
    static ref DBUS_WAITERS: Mutex<BTreeMap<String, WaitQueue>> = Mutex::new(BTreeMap::new());
    static ref DBUS_SESSION_SERVICES: Mutex<BTreeMap<String, SessionServiceEntry>> =
        Mutex::new(BTreeMap::new());
}

pub fn register_session_service(name: &str, auto_restart: bool) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("service name empty");
    }

    let mut services = DBUS_SESSION_SERVICES.lock();
    if services.contains_key(name) {
        return Err("service already registered");
    }

    services.insert(
        name.into(),
        SessionServiceEntry {
            state: SessionServiceState::Starting,
            auto_restart,
            restart_count: 0,
            last_heartbeat_tick: 0,
        },
    );
    counter_inc!(DBUS_SERVICE_REGISTRATIONS);
    Ok(())
}

pub fn mark_session_service_ready(name: &str) -> Result<(), &'static str> {
    let mut services = DBUS_SESSION_SERVICES.lock();
    let service = services.get_mut(name).ok_or("service not found")?;
    service.state = SessionServiceState::Ready;
    Ok(())
}

pub fn heartbeat_session_service(name: &str, tick: u64) -> Result<(), &'static str> {
    let mut services = DBUS_SESSION_SERVICES.lock();
    let service = services.get_mut(name).ok_or("service not found")?;
    service.last_heartbeat_tick = tick;
    counter_inc!(DBUS_SERVICE_HEARTBEATS);
    Ok(())
}

pub fn mark_session_service_degraded(name: &str) -> Result<(), &'static str> {
    let mut services = DBUS_SESSION_SERVICES.lock();
    let service = services.get_mut(name).ok_or("service not found")?;
    service.state = SessionServiceState::Degraded;
    if service.auto_restart {
        service.restart_count = service.restart_count.saturating_add(1);
        service.state = SessionServiceState::Starting;
    }
    Ok(())
}

pub fn list_session_services() -> Vec<SessionServiceSnapshot> {
    let services = DBUS_SESSION_SERVICES.lock();
    services
        .iter()
        .map(|(name, svc)| SessionServiceSnapshot {
            name: name.clone(),
            state: svc.state,
            auto_restart: svc.auto_restart,
            restart_count: svc.restart_count,
            last_heartbeat_tick: svc.last_heartbeat_tick,
        })
        .collect()
}

pub fn dbus_subscribe(topic: &str) -> Result<(), &'static str> {
    counter_inc!(DBUS_SUBSCRIBE_CALLS);
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
    counter_inc!(DBUS_PUBLISH_CALLS);
    if topic.is_empty() {
        return Err("topic empty");
    }

    {
        let mut queues = DBUS_QUEUES.lock();
        let queue = queues.entry(topic.into()).or_insert_with(VecDeque::new);
        if !bounded_push_bytes(queue, payload, DBUS_TOPIC_QUEUE_LIMIT) {
            counter_inc!(DBUS_PUBLISH_DROPS);
            return Err("topic queue full");
        }
    }

    // Wake one waiting consumer
    let mut waiters_map = DBUS_WAITERS.lock();
    if let Some(wq) = waiters_map.get_mut(topic) {
        wake_one_task(wq);
    }

    Ok(payload.len())
}

pub fn dbus_consume(topic: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    counter_inc!(DBUS_CONSUME_CALLS);

    loop {
        {
            let mut queues = DBUS_QUEUES.lock();
            let queue = queues.get_mut(topic).ok_or("topic not found")?;
            if let Some(frame) = queue.pop_front() {
                let copied = core::cmp::min(frame.len(), out.len());
                out[..copied].copy_from_slice(&frame[..copied]);
                counter_inc!(DBUS_CONSUME_HITS);
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

        suspend_on(&wq);
    }
}

pub fn dbus_stats() -> DbusStats {
    DbusStats {
        subscribe_calls: telemetry::snapshot_u64(&DBUS_SUBSCRIBE_CALLS),
        publish_calls: telemetry::snapshot_u64(&DBUS_PUBLISH_CALLS),
        consume_calls: telemetry::snapshot_u64(&DBUS_CONSUME_CALLS),
        publish_drops: telemetry::snapshot_u64(&DBUS_PUBLISH_DROPS),
        consume_hits: telemetry::snapshot_u64(&DBUS_CONSUME_HITS),
        topics: DBUS_QUEUES.lock().len(),
        session_services: DBUS_SESSION_SERVICES.lock().len(),
        service_registrations: telemetry::snapshot_u64(&DBUS_SERVICE_REGISTRATIONS),
        service_heartbeats: telemetry::snapshot_u64(&DBUS_SERVICE_HEARTBEATS),
    }
}

pub fn dbus_take_stats() -> DbusStats {
    DbusStats {
        subscribe_calls: telemetry::take_u64(&DBUS_SUBSCRIBE_CALLS),
        publish_calls: telemetry::take_u64(&DBUS_PUBLISH_CALLS),
        consume_calls: telemetry::take_u64(&DBUS_CONSUME_CALLS),
        publish_drops: telemetry::take_u64(&DBUS_PUBLISH_DROPS),
        consume_hits: telemetry::take_u64(&DBUS_CONSUME_HITS),
        topics: DBUS_QUEUES.lock().len(),
        session_services: DBUS_SESSION_SERVICES.lock().len(),
        service_registrations: telemetry::take_u64(&DBUS_SERVICE_REGISTRATIONS),
        service_heartbeats: telemetry::take_u64(&DBUS_SERVICE_HEARTBEATS),
    }
}

#[cfg(test)]
mod tests;
