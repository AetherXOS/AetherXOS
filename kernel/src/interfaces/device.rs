/// Device management interfaces.
/// 
/// Traits for discovering, registering, and managing hardware devices
/// in a platform-agnostic way.

use crate::interfaces::KernelResult;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Unique device identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(pub u32);

/// Device type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceType {
    /// Serial communication (UART, RS-232, etc.)
    Serial,
    /// Timer/clock devices
    Timer,
    /// Block storage (disk, SSD, etc.)
    BlockStorage,
    /// Network interface
    Network,
    /// Graphics/display device
    Graphics,
    /// Input device (keyboard, mouse, etc.)
    Input,
    /// Interrupt controller
    InterruptController,
    /// Memory management unit
    MMU,
    /// CPU/processor core
    Processor,
    /// Platform controller (chipset, SoC)
    PlatformController,
    /// Unknown device type
    Unknown,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serial => write!(f, "Serial"),
            Self::Timer => write!(f, "Timer"),
            Self::BlockStorage => write!(f, "BlockStorage"),
            Self::Network => write!(f, "Network"),
            Self::Graphics => write!(f, "Graphics"),
            Self::Input => write!(f, "Input"),
            Self::InterruptController => write!(f, "InterruptController"),
            Self::MMU => write!(f, "MMU"),
            Self::Processor => write!(f, "Processor"),
            Self::PlatformController => write!(f, "PlatformController"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Device state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceState {
    /// Device discovered but not initialized
    Discovered,
    /// Device initialization in progress
    Initializing,
    /// Device ready for use
    Ready,
    /// Device is active (interrupt, DMA, etc. in progress)
    Active,
    /// Device temporarily disabled
    Suspended,
    /// Device has encountered an error
    Error,
    /// Device no longer available
    Removed,
}

/// Device descriptor with basic information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Unique device identifier
    pub id: DeviceId,
    /// Device type
    pub device_type: DeviceType,
    /// Device name
    pub name: String,
    /// Current state
    pub state: DeviceState,
    /// Base address (MMIO or port)
    pub base_address: usize,
    /// Size of address space
    pub address_size: usize,
    /// Interrupt vector (if applicable)
    pub interrupt_vector: Option<u16>,
}

/// Trait for registering and discovering devices
pub trait DeviceRegistry: Send + Sync {
    /// Register a discovered device
    fn register(&self, info: DeviceInfo) -> KernelResult<DeviceId>;

    /// Unregister a device
    fn unregister(&self, id: DeviceId) -> KernelResult<()>;

    /// Get device info by ID
    fn get_device(&self, id: DeviceId) -> Option<DeviceInfo>;

    /// Find devices by type
    fn find_devices_by_type(&self, device_type: DeviceType) -> Vec<DeviceInfo>;

    /// Get all registered devices
    fn all_devices(&self) -> Vec<DeviceInfo>;

    /// Update device state
    fn set_device_state(&self, id: DeviceId, state: DeviceState) -> KernelResult<()>;
}

/// Trait for managing device initialization and lifecycle
pub trait DeviceManager: Send + Sync {
    /// Initialize a specific device
    fn init_device(&self, id: DeviceId) -> KernelResult<()>;

    /// Initialize all devices of a specific type
    fn init_devices_by_type(&self, device_type: DeviceType) -> KernelResult<()>;

    /// Suspend a device
    fn suspend_device(&self, id: DeviceId) -> KernelResult<()>;

    /// Resume a suspended device
    fn resume_device(&self, id: DeviceId) -> KernelResult<()>;

    /// Remove a device (cleanup and unregister)
    fn remove_device(&self, id: DeviceId) -> KernelResult<()>;

    /// Get device registry
    fn registry(&self) -> &dyn DeviceRegistry;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id_equality() {
        let id1 = DeviceId(42);
        let id2 = DeviceId(42);
        let id3 = DeviceId(43);
        
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(format!("{}", DeviceType::Serial), "Serial");
        assert_eq!(format!("{}", DeviceType::Timer), "Timer");
        assert_eq!(format!("{}", DeviceType::Network), "Network");
    }

    #[test]
    fn test_device_state_transitions() {
        let mut state = DeviceState::Discovered;
        assert_eq!(state, DeviceState::Discovered);
        
        state = DeviceState::Initializing;
        assert_eq!(state, DeviceState::Initializing);
        
        state = DeviceState::Ready;
        assert_eq!(state, DeviceState::Ready);
    }
}
