use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::modules::drivers::{IrqGrant, MmioGrant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverKitEvent {
    Start,
    Stop,
    Interrupt,
    Reset,
    PowerStateChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverLifecycleState {
    Discovered,
    Bound,
    Started,
    Quiesced,
    Stopped,
    Faulted,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverKitRecoveryPolicy {
    pub max_retries: u8,
    pub quarantine_on_fault: bool,
    pub base_recovery_backoff_ticks: u32,
    pub max_recovery_backoff_ticks: u32,
}

impl DriverKitRecoveryPolicy {
    pub const fn conservative() -> Self {
        Self {
            max_retries: 1,
            quarantine_on_fault: true,
            base_recovery_backoff_ticks: 20,
            max_recovery_backoff_ticks: 200,
        }
    }

    pub const fn balanced() -> Self {
        Self {
            max_retries: 3,
            quarantine_on_fault: false,
            base_recovery_backoff_ticks: 10,
            max_recovery_backoff_ticks: 160,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverKitEventQueue {
    capacity: usize,
    events: VecDeque<DriverKitEvent>,
}

impl DriverKitEventQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            events: VecDeque::new(),
        }
    }

    pub fn push(&mut self, event: DriverKitEvent) -> bool {
        if self.events.len() >= self.capacity {
            return false;
        }
        self.events.push_back(event);
        true
    }

    pub fn pop(&mut self) -> Option<DriverKitEvent> {
        self.events.pop_front()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceMatch {
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverBindingRecord {
    pub device: DeviceMatch,
    pub selected_index: usize,
    pub state: DriverLifecycleState,
    pub retry_count: u8,
    pub last_fault_tick: u32,
    pub next_recovery_tick: u32,
    pub recovery_policy: DriverKitRecoveryPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverKitHealthSnapshot {
    pub class_count: usize,
    pub binding_count: usize,
    pub started_count: usize,
    pub faulted_count: usize,
    pub quarantined_count: usize,
    pub dispatch_success_count: u64,
    pub dispatch_failure_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserModeDriverContext {
    pub mmio_windows: Vec<MmioGrant>,
    pub irq_lines: Vec<IrqGrant>,
    pub event_queue_depth: usize,
    pub interrupt_poll_budget: usize,
}

impl UserModeDriverContext {
    pub fn new() -> Self {
        Self {
            mmio_windows: Vec::new(),
            irq_lines: Vec::new(),
            event_queue_depth: 64,
            interrupt_poll_budget: 8,
        }
    }

    pub fn add_mmio(mut self, grant: MmioGrant) -> Self {
        self.mmio_windows.push(grant);
        self
    }

    pub fn add_irq(mut self, irq: IrqGrant) -> Self {
        self.irq_lines.push(irq);
        self
    }
}

impl Default for UserModeDriverContext {
    fn default() -> Self {
        Self::new()
    }
}

pub trait DriverKitClass {
    fn class_name(&self) -> &'static str;
    fn score(&self, device: &DeviceMatch) -> u32;
    fn start(&mut self, context: &UserModeDriverContext) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn on_event(&mut self, event: DriverKitEvent) -> Result<(), String>;
}
