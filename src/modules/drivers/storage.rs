use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::hal::pci::PciDevice;

use super::ahci::Ahci;
use super::block::{BlockDevice, BlockDeviceInfo, BlockDriverKind};
use super::lifecycle::{DriverHealth, DriverLifecycle, PciProbeDriver};
use super::nvme::Nvme;
use super::virtio_block::VirtIoBlock;

pub trait ManagedStorageDriver: BlockDevice + DriverLifecycle {}
impl<T: BlockDevice + DriverLifecycle> ManagedStorageDriver for T {}

pub enum ProbedStorageDriver {
    Nvme(Nvme),
    Ahci(Ahci),
    VirtIoBlock(VirtIoBlock),
}

impl ProbedStorageDriver {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Nvme(driver) => driver.name(),
            Self::Ahci(driver) => driver.name(),
            Self::VirtIoBlock(driver) => driver.name(),
        }
    }

    pub fn kind(&self) -> BlockDriverKind {
        match self {
            Self::Nvme(_) => BlockDriverKind::Nvme,
            Self::Ahci(_) => BlockDriverKind::Ahci,
            Self::VirtIoBlock(_) => BlockDriverKind::VirtIoBlock,
        }
    }

    pub fn init_driver(&mut self) -> Result<(), &'static str> {
        match self {
            Self::Nvme(driver) => driver.init_driver(),
            Self::Ahci(driver) => driver.init_driver(),
            Self::VirtIoBlock(driver) => driver.init_driver(),
        }
    }

    fn into_managed(self) -> Box<dyn ManagedStorageDriver + Send> {
        match self {
            Self::Nvme(driver) => Box::new(driver),
            Self::Ahci(driver) => Box::new(driver),
            Self::VirtIoBlock(driver) => Box::new(driver),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageDependency {
    PciEnumeration,
    MmioMapping,
    IoPort,
}

#[derive(Debug, Clone, Copy)]
pub struct StorageProbeStep {
    pub name: &'static str,
    pub kind: BlockDriverKind,
    pub order: u8,
    pub dependency: StorageDependency,
    probe: fn(&[PciDevice]) -> Option<ProbedStorageDriver>,
}

impl StorageProbeStep {
    pub fn probe(&self, devices: &[PciDevice]) -> Option<ProbedStorageDriver> {
        (self.probe)(devices)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StorageLifecycleSummary {
    pub total: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageRuntimeRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy)]
pub struct StorageRuntimeReadiness {
    pub has_any_driver: bool,
    pub has_failed_driver: bool,
    pub has_degraded_driver: bool,
    pub probe_discovered_any_driver: bool,
    pub init_failures: usize,
    pub risk_level: StorageRuntimeRiskLevel,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StorageProbeReport {
    pub probe_steps: usize,
    pub probed_drivers: usize,
    pub init_success: usize,
    pub init_failures: usize,
}

pub struct StorageManager {
    devices: Vec<Box<dyn ManagedStorageDriver + Send>>,
    probe_report: StorageProbeReport,
}

fn probe_nvme(devices: &[PciDevice]) -> Option<ProbedStorageDriver> {
    Nvme::probe_pci(devices).map(ProbedStorageDriver::Nvme)
}

fn probe_ahci(devices: &[PciDevice]) -> Option<ProbedStorageDriver> {
    Ahci::probe_pci(devices).map(ProbedStorageDriver::Ahci)
}

fn probe_virtio_block(devices: &[PciDevice]) -> Option<ProbedStorageDriver> {
    VirtIoBlock::probe_pci(devices).map(ProbedStorageDriver::VirtIoBlock)
}

const STORAGE_PROBE_PLAN: [StorageProbeStep; 3] = [
    StorageProbeStep {
        name: "nvme",
        kind: BlockDriverKind::Nvme,
        order: 0,
        dependency: StorageDependency::MmioMapping,
        probe: probe_nvme,
    },
    StorageProbeStep {
        name: "ahci",
        kind: BlockDriverKind::Ahci,
        order: 1,
        dependency: StorageDependency::MmioMapping,
        probe: probe_ahci,
    },
    StorageProbeStep {
        name: "virtio-block",
        kind: BlockDriverKind::VirtIoBlock,
        order: 2,
        dependency: StorageDependency::IoPort,
        probe: probe_virtio_block,
    },
];

pub fn storage_runtime_readiness() -> StorageRuntimeReadiness {
    let guard = StorageManager::global().lock();
    if let Some(manager) = guard.as_ref() {
        manager.runtime_readiness()
    } else {
        StorageRuntimeReadiness {
            has_any_driver: false,
            has_failed_driver: false,
            has_degraded_driver: false,
            probe_discovered_any_driver: false,
            init_failures: 0,
            risk_level: StorageRuntimeRiskLevel::High,
        }
    }
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            probe_report: StorageProbeReport::default(),
        }
    }

    pub fn global() -> &'static crate::kernel::sync::IrqSafeMutex<Option<Self>> {
        static GLOBAL_STORAGE: crate::kernel::sync::IrqSafeMutex<Option<StorageManager>> =
            crate::kernel::sync::IrqSafeMutex::new(None);
        &GLOBAL_STORAGE
    }

    pub fn init_global(devices: &[PciDevice]) {
        let manager = Self::probe_and_init(devices);
        *Self::global().lock() = Some(manager);
    }

    pub fn probe_and_init(devices: &[PciDevice]) -> Self {
        let mut manager = Self::new();
        let mut report = StorageProbeReport::default();

        for step in Self::probe_plan() {
            report.probe_steps = report.probe_steps.saturating_add(1);
            if let Some(mut driver) = step.probe(devices) {
                report.probed_drivers = report.probed_drivers.saturating_add(1);
                if driver.init_driver().is_ok() {
                    report.init_success = report.init_success.saturating_add(1);
                } else {
                    report.init_failures = report.init_failures.saturating_add(1);
                }
                manager.push(driver.into_managed());
            }
        }
        manager.probe_report = report;

        manager
    }

    pub fn probe_plan() -> &'static [StorageProbeStep] {
        &STORAGE_PROBE_PLAN
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn infos(&self, out: &mut [BlockDeviceInfo]) -> usize {
        let mut written = 0usize;
        for device in &self.devices {
            if written >= out.len() {
                break;
            }
            out[written] = device.info();
            written += 1;
        }
        written
    }

    pub fn infos_vec(&self) -> Vec<BlockDeviceInfo> {
        let mut infos = Vec::with_capacity(self.devices.len());
        for device in &self.devices {
            infos.push(device.info());
        }
        infos
    }

    pub fn first_by_kind(
        &mut self,
        kind: BlockDriverKind,
    ) -> Option<&mut dyn ManagedStorageDriver> {
        for device in &mut self.devices {
            if device.info().kind == kind {
                return Some(device.as_mut());
            }
        }
        None
    }

    pub fn probe_report(&self) -> StorageProbeReport {
        self.probe_report
    }

    pub fn lifecycle_summary(&self) -> StorageLifecycleSummary {
        let mut healthy = 0usize;
        let mut degraded = 0usize;
        let mut failed = 0usize;

        for device in &self.devices {
            match device.health() {
                DriverHealth::Healthy => healthy += 1,
                DriverHealth::Degraded => degraded += 1,
                DriverHealth::Failed => failed += 1,
            }
        }

        StorageLifecycleSummary {
            total: self.devices.len(),
            healthy,
            degraded,
            failed,
        }
    }

    pub fn runtime_readiness(&self) -> StorageRuntimeReadiness {
        let summary = self.lifecycle_summary();
        let probe = self.probe_report();
        let has_any_driver = summary.total > 0;
        let has_failed_driver = summary.failed > 0;
        let has_degraded_driver = summary.degraded > 0;
        let probe_discovered_any_driver = probe.probed_drivers > 0;

        let risk_level = if !has_any_driver || has_failed_driver || probe.init_failures > 0 {
            StorageRuntimeRiskLevel::High
        } else if has_degraded_driver {
            StorageRuntimeRiskLevel::Medium
        } else {
            StorageRuntimeRiskLevel::Low
        };

        StorageRuntimeReadiness {
            has_any_driver,
            has_failed_driver,
            has_degraded_driver,
            probe_discovered_any_driver,
            init_failures: probe.init_failures,
            risk_level,
        }
    }

    fn push(&mut self, device: Box<dyn ManagedStorageDriver + Send>) {
        self.devices.push(device);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn storage_probe_plan_is_deterministic_and_sorted() {
        let plan = StorageManager::probe_plan();
        assert_eq!(plan[0].kind, BlockDriverKind::Nvme);
        assert_eq!(plan[1].kind, BlockDriverKind::Ahci);
        assert_eq!(plan[2].kind, BlockDriverKind::VirtIoBlock);
        assert!(plan[0].order < plan[1].order && plan[1].order < plan[2].order);
        assert_eq!(plan[0].name, "nvme");
        assert_eq!(plan[1].name, "ahci");
        assert_eq!(plan[2].name, "virtio-block");
    }

    #[test_case]
    fn readiness_defaults_to_high_without_manager() {
        *StorageManager::global().lock() = None;
        let readiness = storage_runtime_readiness();
        assert!(!readiness.has_any_driver);
        assert_eq!(readiness.risk_level, StorageRuntimeRiskLevel::High);
    }

    #[test_case]
    fn readiness_is_high_for_empty_storage_manager() {
        let manager = StorageManager::new();
        let readiness = manager.runtime_readiness();
        assert!(!readiness.has_any_driver);
        assert!(!readiness.probe_discovered_any_driver);
        assert_eq!(readiness.risk_level, StorageRuntimeRiskLevel::High);
    }
}
