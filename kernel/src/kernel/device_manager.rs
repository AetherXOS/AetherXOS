// --- PHASE 4: DEVICE MANAGER IMPLEMENTATION ---
// Concrete device discovery, registration, and lifecycle management

use crate::core::log;
use alloc::format;
use alloc::vec::Vec;
use crate::interfaces::device::{
    DeviceId, DeviceInfo, DeviceManager, DeviceRegistry, DeviceState, DeviceType,
};
use alloc::collections::BTreeMap;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU32, Ordering};

pub(crate) static NEXT_DEVICE_ID: AtomicU32 = AtomicU32::new(1);
use crate::kernel::sync::IrqSafeMutex;

/// Device entry in the registry
struct DeviceEntry {
    info: DeviceInfo,
}

/// Concrete implementation of DeviceRegistry
pub struct ConcreteDeviceRegistry {
    /// Devices indexed by ID
    devices: IrqSafeMutex<BTreeMap<DeviceId, DeviceEntry>>,

    /// Devices indexed by type (for fast type-based lookup)
    devices_by_type: IrqSafeMutex<BTreeMap<DeviceType, Vec<DeviceId>>>,
}

impl ConcreteDeviceRegistry {
    /// Create a new device registry
    pub const fn new() -> Self {
        Self {
            devices: IrqSafeMutex::new(BTreeMap::new()),
            devices_by_type: IrqSafeMutex::new(BTreeMap::new()),
        }
    }
}

impl DeviceRegistry for ConcreteDeviceRegistry {
    /// Register a new device
    fn register(&self, info: DeviceInfo) -> crate::interfaces::KernelResult<DeviceId> {
        let device_id = info.id;
        let device_type = info.device_type;

        log::debug(&format!(
            "Registering device: {} (type: {:?})",
            info.name, device_type
        ));

        let entry = DeviceEntry {
            info,
        };

        self.devices.lock().insert(device_id, entry);

        // Add to type index
        self.devices_by_type
            .lock()
            .entry(device_type)
            .or_insert_with(Vec::new)
            .push(device_id);

        Ok(device_id)
    }

    /// Unregister a device
    fn unregister(&self, id: DeviceId) -> crate::interfaces::KernelResult<()> {
        let mut devices = self.devices.lock();
        if let Some(entry) = devices.remove(&id) {
            let mut by_type = self.devices_by_type.lock();
            if let Some(ids) = by_type.get_mut(&entry.info.device_type) {
                ids.retain(|&x| x != id);
            }
            Ok(())
        } else {
            Err(crate::interfaces::KernelError::NotFound)
        }
    }

    /// Find devices by type
    fn find_devices_by_type(&self, device_type: DeviceType) -> Vec<DeviceInfo> {
        let by_type = self.devices_by_type.lock();
        let devices = self.devices.lock();
        
        by_type.get(&device_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| devices.get(id).map(|e| e.info.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all devices
    fn all_devices(&self) -> Vec<DeviceInfo> {
        self.devices
            .lock()
            .values()
            .map(|e| e.info.clone())
            .collect()
    }

    /// Set device state
    fn set_device_state(&self, device_id: DeviceId, state: DeviceState) -> crate::interfaces::KernelResult<()> {
        if let Some(entry) = self.devices.lock().get_mut(&device_id) {
            entry.info.state = state;
            log::debug(&format!(
                "Device {} state changed to: {:?}",
                device_id.0, state
            ));
            Ok(())
        } else {
            log::error(&format!("Device {} not found", device_id.0));
            Err(crate::interfaces::KernelError::NotFound)
        }
    }

    /// Get device info
    fn get_device(&self, device_id: DeviceId) -> Option<DeviceInfo> {
        self.devices
            .lock()
            .get(&device_id)
            .map(|e| e.info.clone())
    }
}

/// Concrete implementation of DeviceManager
pub struct ConcreteDeviceManager {
    registry: ConcreteDeviceRegistry,
}

impl ConcreteDeviceManager {
    /// Create a new device manager
    pub const fn new() -> Self {
        Self {
            registry: ConcreteDeviceRegistry::new(),
        }
    }

    /// Register a device with automatic ID assignment
    pub fn register_device(&self, device_type: DeviceType, name: &str) -> crate::interfaces::KernelResult<DeviceId> {
        let id = DeviceId(NEXT_DEVICE_ID.fetch_add(1, Ordering::SeqCst));
        let info = DeviceInfo {
            id,
            device_type,
            name: name.to_string(),
            state: crate::interfaces::device::DeviceState::Ready,
            base_address: 0,
            address_size: 0,
            interrupt_vector: None,
        };
        self.registry.register(info)
    }
}

impl DeviceManager for ConcreteDeviceManager {
    /// Initialize a device (e.g., probe drivers, allocate resources)
    fn init_device(&self, device_id: DeviceId) -> crate::interfaces::KernelResult<()> {
        log::info(&format!("Initializing device: {}", device_id.0));

        if let Some(_) = self.registry.get_device(device_id) {
            // Move to Initializing state
            self.registry
                .set_device_state(device_id, DeviceState::Initializing)?;

            log::debug(&format!("Device {} probe complete", device_id.0));
            self.registry
                .set_device_state(device_id, DeviceState::Ready)?;
            Ok(())
        } else {
            Err(crate::interfaces::KernelError::NotFound)
        }
    }

    /// Initialize all devices of a specific type
    fn init_devices_by_type(&self, device_type: DeviceType) -> crate::interfaces::KernelResult<()> {
        let devices = self.registry.find_devices_by_type(device_type);
        for info in devices {
            self.init_device(info.id)?;
        }
        Ok(())
    }

    /// Suspend a device (stop activity, preserve state)
    fn suspend_device(&self, device_id: DeviceId) -> crate::interfaces::KernelResult<()> {
        log::info(&format!("Suspending device: {}", device_id.0));
        self.registry.set_device_state(device_id, DeviceState::Suspended)
    }

    /// Resume a device (restore previous state)
    fn resume_device(&self, device_id: DeviceId) -> crate::interfaces::KernelResult<()> {
        log::info(&format!("Resuming device: {}", device_id.0));
        self.registry.set_device_state(device_id, DeviceState::Ready)
    }

    /// Remove a device from system
    fn remove_device(&self, device_id: DeviceId) -> crate::interfaces::KernelResult<()> {
        log::info(&format!("Removing device: {}", device_id.0));
        self.registry.set_device_state(device_id, DeviceState::Removed)?;
        self.registry.unregister(device_id)
    }

    /// Get access to device registry for queries
    fn registry(&self) -> &dyn DeviceRegistry {
        &self.registry
    }
}

// Global device manager instance
pub static GLOBAL_DEVICE_MANAGER: ConcreteDeviceManager = ConcreteDeviceManager::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_registration() {
        let registry = ConcreteDeviceRegistry::new();
        let dev_id = DeviceId(1);
        let info = DeviceInfo {
            id: dev_id,
            device_type: DeviceType::Timer,
            name: "timer0".to_string(),
            state: DeviceState::Discovered,
            base_address: 0x1000,
            address_size: 0x100,
            interrupt_vector: None,
        };
        
        assert!(registry.register(info).is_ok());
        assert!(registry.get_device(dev_id).is_some());
    }

    #[test]
    fn test_find_devices_by_type() {
        let registry = ConcreteDeviceRegistry::new();
        let id1 = DeviceId(1);
        let id2 = DeviceId(2);
        
        registry.register(DeviceInfo {
            id: id1,
            device_type: DeviceType::Timer,
            name: "timer0".to_string(),
            state: DeviceState::Discovered,
            base_address: 0,
            address_size: 0,
            interrupt_vector: None,
        }).ok();
        
        registry.register(DeviceInfo {
            id: id2,
            device_type: DeviceType::Serial,
            name: "uart0".to_string(),
            state: DeviceState::Discovered,
            base_address: 0,
            address_size: 0,
            interrupt_vector: None,
        }).ok();

        let timers = registry.find_devices_by_type(DeviceType::Timer);
        assert_eq!(timers.len(), 1);
        assert_eq!(timers[0].id, id1);
    }

    #[test]
    fn test_device_state_management() {
        let registry = ConcreteDeviceRegistry::new();
        let dev_id = DeviceId(1);
        
        registry.register(DeviceInfo {
            id: dev_id,
            device_type: DeviceType::Timer,
            name: "timer0".to_string(),
            state: DeviceState::Discovered,
            base_address: 0,
            address_size: 0,
            interrupt_vector: None,
        }).ok();

        assert_eq!(
            registry.get_device(dev_id).unwrap().state,
            DeviceState::Discovered
        );

        registry.set_device_state(dev_id, DeviceState::Ready).ok();
        assert_eq!(registry.get_device(dev_id).unwrap().state, DeviceState::Ready);
    }

    #[test]
    fn test_device_manager_init() {
        let mgr = ConcreteDeviceManager::new();
        let dev_id = DeviceId(1);
        
        mgr.registry().register(DeviceInfo {
            id: dev_id,
            device_type: DeviceType::Timer,
            name: "timer0".to_string(),
            state: DeviceState::Discovered,
            base_address: 0,
            address_size: 0,
            interrupt_vector: None,
        }).ok();

        assert!(mgr.init_device(dev_id).is_ok());
        assert_eq!(
            mgr.registry().get_device(dev_id).unwrap().state,
            DeviceState::Ready
        );
    }
}
