use crate::hal::pci::PciDevice;

use super::lifecycle::{DriverLifecycle, DriverStatus};
use super::{ActiveNetworkDriver, NetworkDriverPolicy, PciProbeDriver, VirtIoNet, E1000};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverBus {
    Pci,
    Mmio,
    IoPort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeDependency {
    PciEnumeration,
    MmioMapping,
    IoPortAccess,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkProbeStep {
    pub name: &'static str,
    pub order: u8,
    pub bus: DriverBus,
    pub dependency: ProbeDependency,
    pub active_kind: ActiveNetworkDriver,
    probe: fn(&[PciDevice]) -> Option<ProbedNetworkDriver>,
}

pub enum ProbedNetworkDriver {
    VirtIo(VirtIoNet),
    E1000(E1000),
}

impl ProbedNetworkDriver {
    pub fn name(&self) -> &'static str {
        match self {
            Self::VirtIo(driver) => driver.name(),
            Self::E1000(driver) => driver.name(),
        }
    }

    pub fn irq_line(&self) -> u8 {
        match self {
            Self::VirtIo(driver) => driver.irq,
            Self::E1000(driver) => driver.irq,
        }
    }

    pub fn active_kind(&self) -> ActiveNetworkDriver {
        match self {
            Self::VirtIo(_) => ActiveNetworkDriver::VirtIo,
            Self::E1000(_) => ActiveNetworkDriver::E1000,
        }
    }

    pub fn init_driver(&mut self) -> Result<(), &'static str> {
        match self {
            Self::VirtIo(driver) => driver.init_driver(),
            Self::E1000(driver) => driver.init_driver(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::VirtIo(driver) => driver.status(),
            Self::E1000(driver) => driver.status(),
        }
    }

    pub fn into_virtio(self) -> Option<VirtIoNet> {
        match self {
            Self::VirtIo(driver) => Some(driver),
            Self::E1000(_) => None,
        }
    }

    pub fn into_e1000(self) -> Option<E1000> {
        match self {
            Self::VirtIo(_) => None,
            Self::E1000(driver) => Some(driver),
        }
    }
}

impl NetworkProbeStep {
    pub fn probe(&self, devices: &[PciDevice]) -> Option<ProbedNetworkDriver> {
        (self.probe)(devices)
    }
}

fn probe_virtio(devices: &[PciDevice]) -> Option<ProbedNetworkDriver> {
    VirtIoNet::probe_pci(devices).map(ProbedNetworkDriver::VirtIo)
}

fn probe_e1000(devices: &[PciDevice]) -> Option<ProbedNetworkDriver> {
    E1000::probe_pci(devices).map(ProbedNetworkDriver::E1000)
}

const NETWORK_PROBE_PLAN: [NetworkProbeStep; 2] = [
    NetworkProbeStep {
        name: "virtio-net",
        order: 0,
        bus: DriverBus::Pci,
        dependency: ProbeDependency::IoPortAccess,
        active_kind: ActiveNetworkDriver::VirtIo,
        probe: probe_virtio,
    },
    NetworkProbeStep {
        name: "e1000",
        order: 1,
        bus: DriverBus::Pci,
        dependency: ProbeDependency::MmioMapping,
        active_kind: ActiveNetworkDriver::E1000,
        probe: probe_e1000,
    },
];

pub fn network_probe_plan() -> &'static [NetworkProbeStep] {
    &NETWORK_PROBE_PLAN
}

pub fn probe_first_network_driver(devices: &[PciDevice]) -> Option<ProbedNetworkDriver> {
    probe_network_driver_with_policy(devices, NetworkDriverPolicy::PreferVirtIo)
}

fn probe_network_kind(
    devices: &[PciDevice],
    kind: ActiveNetworkDriver,
) -> Option<ProbedNetworkDriver> {
    for step in network_probe_plan() {
        if step.active_kind != kind {
            continue;
        }
        if let Some(driver) = step.probe(devices) {
            return Some(driver);
        }
    }
    None
}

pub fn probe_network_driver_with_policy(
    devices: &[PciDevice],
    policy: NetworkDriverPolicy,
) -> Option<ProbedNetworkDriver> {
    match policy {
        NetworkDriverPolicy::PreferVirtIo => {
            probe_network_kind(devices, ActiveNetworkDriver::VirtIo)
                .or_else(|| probe_network_kind(devices, ActiveNetworkDriver::E1000))
        }
        NetworkDriverPolicy::PreferE1000 => probe_network_kind(devices, ActiveNetworkDriver::E1000)
            .or_else(|| probe_network_kind(devices, ActiveNetworkDriver::VirtIo)),
        NetworkDriverPolicy::VirtIoOnly => probe_network_kind(devices, ActiveNetworkDriver::VirtIo),
        NetworkDriverPolicy::E1000Only => probe_network_kind(devices, ActiveNetworkDriver::E1000),
    }
}

pub fn probe_policy_fallback_kind(policy: NetworkDriverPolicy) -> ActiveNetworkDriver {
    match policy {
        NetworkDriverPolicy::VirtIoOnly => ActiveNetworkDriver::None,
        NetworkDriverPolicy::E1000Only => ActiveNetworkDriver::None,
        NetworkDriverPolicy::PreferVirtIo => ActiveNetworkDriver::E1000,
        NetworkDriverPolicy::PreferE1000 => ActiveNetworkDriver::VirtIo,
    }
}

pub fn probe_policy_primary_kind(policy: NetworkDriverPolicy) -> ActiveNetworkDriver {
    match policy {
        NetworkDriverPolicy::PreferVirtIo | NetworkDriverPolicy::VirtIoOnly => {
            ActiveNetworkDriver::VirtIo
        }
        NetworkDriverPolicy::PreferE1000 | NetworkDriverPolicy::E1000Only => {
            ActiveNetworkDriver::E1000
        }
    }
}

pub fn probe_first_network_driver_default_policy(
    devices: &[PciDevice],
) -> Option<ProbedNetworkDriver> {
    let policy = super::network_driver_policy();
    probe_network_driver_with_policy(devices, policy)
}

pub fn probe_first_network_driver_plan_order(devices: &[PciDevice]) -> Option<ProbedNetworkDriver> {
    for step in network_probe_plan() {
        if let Some(driver) = step.probe(devices) {
            return Some(driver);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn network_probe_plan_order_is_stable() {
        let plan = network_probe_plan();
        assert_eq!(plan[0].name, "virtio-net");
        assert_eq!(plan[1].name, "e1000");
        assert!(plan[0].order < plan[1].order);
    }

    #[test_case]
    fn probe_policy_primary_and_fallback_are_consistent() {
        assert_eq!(
            probe_policy_primary_kind(NetworkDriverPolicy::PreferVirtIo),
            ActiveNetworkDriver::VirtIo
        );
        assert_eq!(
            probe_policy_fallback_kind(NetworkDriverPolicy::PreferVirtIo),
            ActiveNetworkDriver::E1000
        );
        assert_eq!(
            probe_policy_primary_kind(NetworkDriverPolicy::PreferE1000),
            ActiveNetworkDriver::E1000
        );
        assert_eq!(
            probe_policy_fallback_kind(NetworkDriverPolicy::PreferE1000),
            ActiveNetworkDriver::VirtIo
        );
        assert_eq!(
            probe_policy_fallback_kind(NetworkDriverPolicy::VirtIoOnly),
            ActiveNetworkDriver::None
        );
    }
}
