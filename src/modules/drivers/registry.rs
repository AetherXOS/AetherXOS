use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use super::{ActiveNetworkDriver, ProbedNetworkDriver, VirtIoNet, E1000};

pub const RUNTIME_REGISTRY_EVENT_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverRuntimeEventKind {
    Registered,
    Unregistered,
    HotplugAttached,
    HotplugDetached,
    RebindSucceeded,
    RebindFailed,
    FailoverActivated,
    PolicySwitched,
    Quarantined,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverRuntimeEvent {
    pub seq: u64,
    pub kind: DriverRuntimeEventKind,
    pub driver: ActiveNetworkDriver,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverRuntimeRegistrySnapshot {
    pub has_virtio: bool,
    pub has_e1000: bool,
    pub register_calls: u64,
    pub unregister_calls: u64,
    pub hotplug_attach_calls: u64,
    pub hotplug_detach_calls: u64,
    pub last_attach: ActiveNetworkDriver,
    pub last_detach: ActiveNetworkDriver,
    pub event_count: usize,
    pub event_overwrites: u64,
    pub last_event: Option<DriverRuntimeEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverRuntimeRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverRuntimeReadiness {
    pub has_any_driver: bool,
    pub can_failover: bool,
    pub active_driver: ActiveNetworkDriver,
    pub active_driver_registered: bool,
    pub risk_level: DriverRuntimeRiskLevel,
}

struct RuntimeRegistryEventRing {
    events: [Option<DriverRuntimeEvent>; RUNTIME_REGISTRY_EVENT_CAPACITY],
    write_index: usize,
    len: usize,
    overwrites: u64,
}

impl RuntimeRegistryEventRing {
    const fn new() -> Self {
        Self {
            events: [None; RUNTIME_REGISTRY_EVENT_CAPACITY],
            write_index: 0,
            len: 0,
            overwrites: 0,
        }
    }

    fn push(&mut self, event: DriverRuntimeEvent) {
        if self.len == RUNTIME_REGISTRY_EVENT_CAPACITY {
            self.overwrites = self.overwrites.saturating_add(1);
        } else {
            self.len += 1;
        }
        self.events[self.write_index] = Some(event);
        self.write_index = (self.write_index + 1) % RUNTIME_REGISTRY_EVENT_CAPACITY;
    }

    fn latest(&self) -> Option<DriverRuntimeEvent> {
        if self.len == 0 {
            return None;
        }
        let idx = if self.write_index == 0 {
            RUNTIME_REGISTRY_EVENT_CAPACITY - 1
        } else {
            self.write_index - 1
        };
        self.events[idx]
    }

    fn recent_into(&self, out: &mut [DriverRuntimeEvent]) -> usize {
        if self.len == 0 || out.is_empty() {
            return 0;
        }
        let n = core::cmp::min(self.len, out.len());
        let oldest = if self.len == RUNTIME_REGISTRY_EVENT_CAPACITY {
            self.write_index
        } else {
            0
        };
        let start = self.len.saturating_sub(n);
        let mut written = 0usize;
        let mut cursor = (oldest + start) % RUNTIME_REGISTRY_EVENT_CAPACITY;
        while written < n {
            if let Some(event) = self.events[cursor] {
                out[written] = event;
                written += 1;
            }
            cursor = (cursor + 1) % RUNTIME_REGISTRY_EVENT_CAPACITY;
        }
        written
    }
}

static VIRTIO_RUNTIME_DRIVER: Mutex<Option<VirtIoNet>> = Mutex::new(None);
static E1000_RUNTIME_DRIVER: Mutex<Option<E1000>> = Mutex::new(None);
static REGISTER_CALLS: AtomicU64 = AtomicU64::new(0);
static UNREGISTER_CALLS: AtomicU64 = AtomicU64::new(0);
static HOTPLUG_ATTACH_CALLS: AtomicU64 = AtomicU64::new(0);
static HOTPLUG_DETACH_CALLS: AtomicU64 = AtomicU64::new(0);
static LAST_ATTACH_DRIVER: AtomicU64 = AtomicU64::new(0);
static LAST_DETACH_DRIVER: AtomicU64 = AtomicU64::new(0);
static EVENT_SEQ: AtomicU64 = AtomicU64::new(0);
static RUNTIME_EVENTS: Mutex<RuntimeRegistryEventRing> =
    Mutex::new(RuntimeRegistryEventRing::new());

#[inline(always)]
const fn driver_to_raw(driver: ActiveNetworkDriver) -> u64 {
    match driver {
        ActiveNetworkDriver::None => 0,
        ActiveNetworkDriver::VirtIo => 1,
        ActiveNetworkDriver::E1000 => 2,
    }
}

#[inline(always)]
const fn driver_from_raw(raw: u64) -> ActiveNetworkDriver {
    match raw {
        1 => ActiveNetworkDriver::VirtIo,
        2 => ActiveNetworkDriver::E1000,
        _ => ActiveNetworkDriver::None,
    }
}

pub fn clear_network_runtime_registry() {
    *VIRTIO_RUNTIME_DRIVER.lock() = None;
    *E1000_RUNTIME_DRIVER.lock() = None;
}

fn record_event(kind: DriverRuntimeEventKind, driver: ActiveNetworkDriver) {
    let seq = EVENT_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let mut events = RUNTIME_EVENTS.lock();
    events.push(DriverRuntimeEvent { seq, kind, driver });
}

pub fn register_network_runtime_driver(driver: ProbedNetworkDriver) -> ActiveNetworkDriver {
    REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    let kind = match driver {
        ProbedNetworkDriver::VirtIo(net) => {
            *VIRTIO_RUNTIME_DRIVER.lock() = Some(net);
            ActiveNetworkDriver::VirtIo
        }
        ProbedNetworkDriver::E1000(net) => {
            *E1000_RUNTIME_DRIVER.lock() = Some(net);
            ActiveNetworkDriver::E1000
        }
    };
    record_event(DriverRuntimeEventKind::Registered, kind);
    kind
}

pub fn unregister_network_runtime_driver(kind: ActiveNetworkDriver) -> bool {
    UNREGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    let removed = match kind {
        ActiveNetworkDriver::VirtIo => VIRTIO_RUNTIME_DRIVER.lock().take().is_some(),
        ActiveNetworkDriver::E1000 => E1000_RUNTIME_DRIVER.lock().take().is_some(),
        ActiveNetworkDriver::None => false,
    };
    if removed || matches!(kind, ActiveNetworkDriver::None) {
        record_event(DriverRuntimeEventKind::Unregistered, kind);
    }
    removed
}

pub fn hotplug_attach_network_driver(driver: ProbedNetworkDriver) -> ActiveNetworkDriver {
    HOTPLUG_ATTACH_CALLS.fetch_add(1, Ordering::Relaxed);
    let kind = driver.active_kind();
    LAST_ATTACH_DRIVER.store(driver_to_raw(kind), Ordering::Relaxed);
    let attached = register_network_runtime_driver(driver);
    record_event(DriverRuntimeEventKind::HotplugAttached, attached);
    attached
}

pub fn hotplug_detach_network_driver(kind: ActiveNetworkDriver) -> bool {
    HOTPLUG_DETACH_CALLS.fetch_add(1, Ordering::Relaxed);
    LAST_DETACH_DRIVER.store(driver_to_raw(kind), Ordering::Relaxed);
    let was_active = super::network::active_driver() == kind;
    if was_active {
        super::network::set_driver_io_owned(false);
        let _ = super::network::clear_driver_queues(kind);
    }
    let removed = unregister_network_runtime_driver(kind);
    if removed || matches!(kind, ActiveNetworkDriver::None) {
        record_event(DriverRuntimeEventKind::HotplugDetached, kind);
    }
    if was_active {
        let promoted = match kind {
            ActiveNetworkDriver::VirtIo if has_e1000_runtime_driver() => {
                Some(ActiveNetworkDriver::E1000)
            }
            ActiveNetworkDriver::E1000 if has_virtio_runtime_driver() => {
                Some(ActiveNetworkDriver::VirtIo)
            }
            _ => None,
        };
        if let Some(driver) = promoted {
            match driver {
                ActiveNetworkDriver::VirtIo => super::network::register_virtio(),
                ActiveNetworkDriver::E1000 => super::network::register_e1000(),
                ActiveNetworkDriver::None => {}
            }
            super::network::set_driver_io_owned(true);
            record_event(DriverRuntimeEventKind::FailoverActivated, driver);
        } else {
            super::network::clear_active_driver();
            record_event(
                DriverRuntimeEventKind::FailoverActivated,
                ActiveNetworkDriver::None,
            );
        }
    }
    removed
}

pub fn note_rebind_result(kind: ActiveNetworkDriver, ok: bool) {
    if ok {
        record_event(DriverRuntimeEventKind::RebindSucceeded, kind);
    } else {
        record_event(DriverRuntimeEventKind::RebindFailed, kind);
    }
}

pub fn note_policy_switch(kind: ActiveNetworkDriver) {
    record_event(DriverRuntimeEventKind::PolicySwitched, kind);
}

pub fn note_quarantine(kind: ActiveNetworkDriver) {
    record_event(DriverRuntimeEventKind::Quarantined, kind);
}

pub fn has_virtio_runtime_driver() -> bool {
    VIRTIO_RUNTIME_DRIVER.lock().is_some()
}

pub fn has_e1000_runtime_driver() -> bool {
    E1000_RUNTIME_DRIVER.lock().is_some()
}

pub fn with_virtio_runtime_driver_mut<R>(f: impl FnOnce(&mut VirtIoNet) -> R) -> Option<R> {
    let mut guard = VIRTIO_RUNTIME_DRIVER.try_lock()?;
    let driver = guard.as_mut()?;
    Some(f(driver))
}

pub fn with_e1000_runtime_driver_mut<R>(f: impl FnOnce(&mut E1000) -> R) -> Option<R> {
    let mut guard = E1000_RUNTIME_DRIVER.try_lock()?;
    let driver = guard.as_mut()?;
    Some(f(driver))
}

pub fn runtime_registry_snapshot() -> DriverRuntimeRegistrySnapshot {
    let events = RUNTIME_EVENTS.lock();
    DriverRuntimeRegistrySnapshot {
        has_virtio: has_virtio_runtime_driver(),
        has_e1000: has_e1000_runtime_driver(),
        register_calls: REGISTER_CALLS.load(Ordering::Relaxed),
        unregister_calls: UNREGISTER_CALLS.load(Ordering::Relaxed),
        hotplug_attach_calls: HOTPLUG_ATTACH_CALLS.load(Ordering::Relaxed),
        hotplug_detach_calls: HOTPLUG_DETACH_CALLS.load(Ordering::Relaxed),
        last_attach: driver_from_raw(LAST_ATTACH_DRIVER.load(Ordering::Relaxed)),
        last_detach: driver_from_raw(LAST_DETACH_DRIVER.load(Ordering::Relaxed)),
        event_count: events.len,
        event_overwrites: events.overwrites,
        last_event: events.latest(),
    }
}

pub fn runtime_registry_events(out: &mut [DriverRuntimeEvent]) -> usize {
    let events = RUNTIME_EVENTS.lock();
    events.recent_into(out)
}

pub fn latest_runtime_registry_event() -> Option<DriverRuntimeEvent> {
    RUNTIME_EVENTS.lock().latest()
}

pub fn runtime_readiness() -> DriverRuntimeReadiness {
    let has_virtio = has_virtio_runtime_driver();
    let has_e1000 = has_e1000_runtime_driver();
    let has_any_driver = has_virtio || has_e1000;
    let can_failover = has_virtio && has_e1000;
    let active_driver = super::network::active_driver();
    let active_driver_registered = match active_driver {
        ActiveNetworkDriver::None => !has_any_driver,
        ActiveNetworkDriver::VirtIo => has_virtio,
        ActiveNetworkDriver::E1000 => has_e1000,
    };

    let risk_level = if !has_any_driver || !active_driver_registered {
        DriverRuntimeRiskLevel::High
    } else if !can_failover {
        DriverRuntimeRiskLevel::Medium
    } else {
        DriverRuntimeRiskLevel::Low
    };

    DriverRuntimeReadiness {
        has_any_driver,
        can_failover,
        active_driver,
        active_driver_registered,
        risk_level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn detach_none_is_safe_and_records_event() {
        clear_network_runtime_registry();
        let before = runtime_registry_snapshot();
        assert!(!hotplug_detach_network_driver(ActiveNetworkDriver::None));
        let after = runtime_registry_snapshot();
        assert!(after.hotplug_detach_calls >= before.hotplug_detach_calls);
        assert_eq!(after.last_detach, ActiveNetworkDriver::None);
    }

    #[test_case]
    fn rebind_events_are_recorded_in_recent_stream() {
        note_rebind_result(ActiveNetworkDriver::VirtIo, true);
        note_rebind_result(ActiveNetworkDriver::E1000, false);
        let mut events = [DriverRuntimeEvent {
            seq: 0,
            kind: DriverRuntimeEventKind::Registered,
            driver: ActiveNetworkDriver::None,
        }; 4];
        let count = runtime_registry_events(&mut events);
        assert!(count >= 2);
        let tail = &events[count - 1];
        assert_eq!(tail.kind, DriverRuntimeEventKind::RebindFailed);
        assert_eq!(tail.driver, ActiveNetworkDriver::E1000);
    }

    #[test_case]
    fn policy_switch_event_is_recorded() {
        note_policy_switch(ActiveNetworkDriver::VirtIo);
        let latest = latest_runtime_registry_event();
        assert!(latest.is_some());
        let event = latest.unwrap();
        assert_eq!(event.kind, DriverRuntimeEventKind::PolicySwitched);
        assert_eq!(event.driver, ActiveNetworkDriver::VirtIo);
    }

    #[test_case]
    fn readiness_high_when_no_driver_registered() {
        clear_network_runtime_registry();
        super::super::network::clear_active_driver();
        let readiness = runtime_readiness();
        assert!(!readiness.has_any_driver);
        assert!(!readiness.can_failover);
        assert_eq!(readiness.risk_level, DriverRuntimeRiskLevel::High);
    }
}
