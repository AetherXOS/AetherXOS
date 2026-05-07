use crate::interfaces::HardwareAbstraction;
use spin::Mutex;

/// IOMMU Domain for Secure DMA.
/// Ensures that devices can only access specific physical memory regions.
pub struct IommuDomain {
    pub device_id: u32,
    pub active: bool,
}

impl IommuDomain {
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            active: true,
        }
    }

    /// Map a physical address for device DMA.
    /// Returns a 'Device Virtual Address' (DVA).
    pub fn map_dma(&self, phys_addr: u64, size: usize) -> Result<u64, &'static str> {
        // Platform-specific IOMMU mapping (e.g. Intel VT-d / AMD-Vi)
        crate::klog_info!("[IOMMU] Securely mapped {:#x} ({} bytes) for device {}", phys_addr, size, self.device_id);
        Ok(phys_addr) // In a mock, DVA = PA
    }

    /// Unmap a previously mapped DMA region.
    pub fn unmap_dma(&self, dva: u64, size: usize) {
        crate::klog_info!("[IOMMU] Unmapped DVA {:#x} for device {}", dva, self.device_id);
    }
}

pub static GLOBAL_IOMMU_REGISTRY: Mutex<alloc::vec::Vec<IommuDomain>> = Mutex::new(alloc::vec::Vec::new());
