use crate::interfaces::task::TaskId;
use crate::interfaces::scheduler_ext::SchedulingPolicy;
use crate::modules::drivers::hybrid::DriverCapabilitySet;
use crate::modules::ipc::LockFreeRingBuffer;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnclaveConfig {
    pub virtual_memory_limit: usize,
    pub capabilities: DriverCapabilitySet,
    pub scheduling_policy: SchedulingPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnclaveState {
    Uninitialized,
    Running,
    Suspended,
    Terminated,
}

pub struct DriverEnclave {
    pub enclave_id: u32,
    pub config: EnclaveConfig,
    pub state: Mutex<EnclaveState>,
    pub task_id: Mutex<Option<TaskId>>,
    pub command_ring: Arc<LockFreeRingBuffer>,
}

impl DriverEnclave {
    pub fn new(enclave_id: u32, config: EnclaveConfig) -> Self {
        Self {
            enclave_id,
            config,
            state: Mutex::new(EnclaveState::Uninitialized),
            task_id: Mutex::new(None),
            command_ring: Arc::new(LockFreeRingBuffer::new()),
        }
    }

    /// Check if the enclave possesses a required capability.
    pub fn check_capability(&self, cap: DriverCapabilitySet) -> bool {
        self.config.capabilities.contains(cap)
    }

    /// Verify MMIO access permission.
    pub fn permit_mmio(&self, _base: usize, _len: usize) -> bool {
        self.check_capability(DriverCapabilitySet::MMIO)
    }

    /// Verify DMA access permission.
    pub fn permit_dma(&self, _iova: usize, _len: usize) -> bool {
        self.check_capability(DriverCapabilitySet::DMA)
    }

    /// Verify IRQ access permission.
    pub fn permit_irq(&self, _vector: u32) -> bool {
        self.check_capability(DriverCapabilitySet::IRQ)
    }
}

pub struct EnclaveManager {
    enclaves: Mutex<BTreeMap<u32, Arc<DriverEnclave>>>,
    next_id: Mutex<u32>,
}

impl EnclaveManager {
    pub fn new() -> Self {
        Self {
            enclaves: Mutex::new(BTreeMap::new()),
            next_id: Mutex::new(1),
        }
    }

    /// Create and register a new driver enclave.
    pub fn create_enclave(&self, config: EnclaveConfig) -> Arc<DriverEnclave> {
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

        let enclave = Arc::new(DriverEnclave::new(id, config));
        self.enclaves.lock().insert(id, enclave.clone());
        enclave
    }

    /// Retrieve an enclave by ID.
    pub fn get_enclave(&self, id: u32) -> Option<Arc<DriverEnclave>> {
        self.enclaves.lock().get(&id).cloned()
    }

    /// Terminate an enclave and remove it from registry.
    pub fn terminate_enclave(&self, id: u32) -> bool {
        if let Some(enclave) = self.get_enclave(id) {
            let mut state = enclave.state.lock();
            *state = EnclaveState::Terminated;
            self.enclaves.lock().remove(&id);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod enclave_device_test;
