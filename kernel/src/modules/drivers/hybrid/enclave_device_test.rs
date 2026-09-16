/// Ring 3 Enclave Device Node IPC Delegation Tests
///
/// Tests for zero-copy IPC delegation of `/dev/enclave_kbd` to a Ring-3
/// driver enclave via `LockFreeRingBuffer`.
use super::*;
use crate::modules::ipc::LockFreeRingBuffer;
use alloc::sync::Arc;

/// Simulated keyboard event — 4-byte key code + 1-byte action.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct KbdEvent {
    key_code: u32,
    action: u8,
}

impl KbdEvent {
    const ACTION_PRESS: u8 = 1;
    const ACTION_RELEASE: u8 = 0;

    const fn new(key_code: u32, action: u8) -> Self {
        Self { key_code, action }
    }

    fn to_bytes(self) -> [u8; 5] {
        let kc = self.key_code.to_le_bytes();
        [kc[0], kc[1], kc[2], kc[3], self.action]
    }

    fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < 5 {
            return None;
        }
        let key_code = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        Some(Self { key_code, action: b[4] })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct UserspaceIoStats {
    events_sent: usize,
    events_processed: usize,
    buffer_full_retries: usize,
}

struct EnclaveKbdNode {
    ring: Arc<LockFreeRingBuffer>,
}

impl EnclaveKbdNode {
    fn new(ring: Arc<LockFreeRingBuffer>) -> Self {
        Self { ring }
    }

    fn write_event(&self, event: KbdEvent, stats: &mut UserspaceIoStats) -> bool {
        let bytes = event.to_bytes();
        if self.ring.try_send(&bytes) {
            stats.events_sent += 1;
            true
        } else {
            stats.buffer_full_retries += 1;
            false
        }
    }
}

fn enclave_kbd_driver_loop(
    ring: Arc<LockFreeRingBuffer>,
    stats: Arc<spin::Mutex<UserspaceIoStats>>,
    max_idle: usize,
) -> alloc::vec::Vec<KbdEvent> {
    let mut buf = [0u8; 64];
    let mut received = alloc::vec::Vec::new();
    let mut idle = 0usize;
    loop {
        if let Some(n) = ring.try_recv(&mut buf) {
            if let Some(evt) = KbdEvent::from_bytes(&buf[..n]) {
                received.push(evt);
                stats.lock().events_processed += 1;
            }
            idle = 0;
        } else {
            idle += 1;
            if idle >= max_idle { break; }
            core::hint::spin_loop();
        }
    }
    received
}

#[test_case]
fn enclave_kbd_zero_copy_roundtrip() {
    let ring = Arc::new(LockFreeRingBuffer::new());
    let node = EnclaveKbdNode::new(ring.clone());
    let stats = Arc::new(spin::Mutex::new(UserspaceIoStats::default()));
    let mut local = UserspaceIoStats::default();

    let event = KbdEvent::new(0x41, KbdEvent::ACTION_PRESS);
    assert!(node.write_event(event, &mut local));
    assert_eq!(local.events_sent, 1);

    let received = enclave_kbd_driver_loop(ring, stats.clone(), 1);
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], event);
    assert_eq!(stats.lock().events_processed, 1);
}

#[test_case]
fn enclave_kbd_event_ordering() {
    let ring = Arc::new(LockFreeRingBuffer::new());
    let node = EnclaveKbdNode::new(ring.clone());
    let stats = Arc::new(spin::Mutex::new(UserspaceIoStats::default()));
    let mut local = UserspaceIoStats::default();

    let events = [
        KbdEvent::new(0x41, KbdEvent::ACTION_PRESS),
        KbdEvent::new(0x41, KbdEvent::ACTION_RELEASE),
        KbdEvent::new(0x42, KbdEvent::ACTION_PRESS),
        KbdEvent::new(0x42, KbdEvent::ACTION_RELEASE),
    ];
    for evt in events.iter() {
        assert!(node.write_event(*evt, &mut local));
    }
    assert_eq!(local.events_sent, 4);

    let received = enclave_kbd_driver_loop(ring, stats.clone(), 4);
    assert_eq!(received.len(), 4);
    for (i, evt) in events.iter().enumerate() {
        assert_eq!(&received[i], evt, "ordering mismatch at {i}");
    }
    assert_eq!(stats.lock().events_processed, 4);
}

#[test_case]
fn enclave_manager_keyboard_enclave_lifecycle() {
    use crate::modules::drivers::hybrid::DriverCapabilitySet;
    use crate::interfaces::scheduler_ext::SchedulingPolicy;

    let mgr = EnclaveManager::new();
    let config = EnclaveConfig {
        virtual_memory_limit: 4 * 1024 * 1024,
        capabilities: DriverCapabilitySet::IRQ | DriverCapabilitySet::SHARED_MEMORY,
        scheduling_policy: SchedulingPolicy::CFS,
    };
    let enclave = mgr.create_enclave(config);

    assert!(enclave.check_capability(DriverCapabilitySet::IRQ));
    assert!(enclave.check_capability(DriverCapabilitySet::SHARED_MEMORY));
    assert!(!enclave.check_capability(DriverCapabilitySet::MMIO));
    assert!(!enclave.check_capability(DriverCapabilitySet::DMA));

    let id = enclave.enclave_id;
    assert!(mgr.terminate_enclave(id));
    assert!(mgr.get_enclave(id).is_none());
}

#[test_case]
fn enclave_kbd_full_ring_reports_retry() {
    let ring = Arc::new(LockFreeRingBuffer::new());
    let node = EnclaveKbdNode::new(ring.clone());
    let mut local = UserspaceIoStats::default();

    let event = KbdEvent::new(0x20, KbdEvent::ACTION_PRESS);
    let mut sent = 0usize;
    while node.write_event(event, &mut local) {
        sent += 1;
        if sent > 4096 { break; }
    }
    assert!(local.events_sent > 0);
    assert!(local.buffer_full_retries > 0);
}
