use alloc::vec::Vec;

use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::{
    BackendPreference, HybridFeatureKind, HybridRequest, HybridRequestFamily, HybridRequestKind,
};

pub(crate) fn all_feature_kinds() -> [HybridFeatureKind; 10] {
    [
        HybridFeatureKind::Mmio,
        HybridFeatureKind::Dma,
        HybridFeatureKind::Irq,
        HybridFeatureKind::SharedMemory,
        HybridFeatureKind::ControlQueue,
        HybridFeatureKind::Reset,
        HybridFeatureKind::Hotplug,
        HybridFeatureKind::PowerManagement,
        HybridFeatureKind::Snapshot,
        HybridFeatureKind::LiveMigration,
    ]
}

pub(crate) const ALL_REQUEST_KINDS: [HybridRequestKind; 29] = [
    HybridRequestKind::Network,
    HybridRequestKind::Block,
    HybridRequestKind::Ethernet,
    HybridRequestKind::Storage,
    HybridRequestKind::Modem,
    HybridRequestKind::Printer,
    HybridRequestKind::Rtc,
    HybridRequestKind::SensorHub,
    HybridRequestKind::Gpu,
    HybridRequestKind::WiFi,
    HybridRequestKind::Camera,
    HybridRequestKind::Audio,
    HybridRequestKind::Sensor,
    HybridRequestKind::Input,
    HybridRequestKind::Touch,
    HybridRequestKind::Gamepad,
    HybridRequestKind::Bluetooth,
    HybridRequestKind::Nfc,
    HybridRequestKind::Tpm,
    HybridRequestKind::Dock,
    HybridRequestKind::Display,
    HybridRequestKind::Usb,
    HybridRequestKind::Can,
    HybridRequestKind::Serial,
    HybridRequestKind::Firmware,
    HybridRequestKind::SmartCard,
    HybridRequestKind::Nvme,
    HybridRequestKind::WindowsPe,
    HybridRequestKind::UserModeDevice,
];

pub(crate) fn supported_features_for(
    request_kind: HybridRequestKind,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> Vec<HybridFeatureKind> {
    let mut supported = match backend {
        BackendPreference::SideCarFirst => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Dma,
            HybridFeatureKind::Irq,
            HybridFeatureKind::SharedMemory,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
            HybridFeatureKind::Hotplug,
            HybridFeatureKind::PowerManagement,
            HybridFeatureKind::Snapshot,
        ],
        BackendPreference::LibLinuxFirst => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Dma,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
            HybridFeatureKind::PowerManagement,
        ],
        BackendPreference::DriverKitFirst => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
            HybridFeatureKind::Hotplug,
            HybridFeatureKind::PowerManagement,
        ],
        BackendPreference::ReactOsFirst => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
            HybridFeatureKind::Snapshot,
        ],
    };

    if backend == BackendPreference::SideCarFirst
        && matches!(
            request_kind,
            HybridRequestKind::Network
                | HybridRequestKind::Block
                | HybridRequestKind::Ethernet
                | HybridRequestKind::Storage
                | HybridRequestKind::Modem
                | HybridRequestKind::WiFi
                | HybridRequestKind::Rtc
                | HybridRequestKind::SensorHub
        )
    {
        supported.push(HybridFeatureKind::LiveMigration);
    }

    if backend == BackendPreference::ReactOsFirst && request_kind == HybridRequestKind::WindowsPe {
        if !supported.contains(&HybridFeatureKind::LiveMigration) {
            supported.push(HybridFeatureKind::LiveMigration);
        }
    }

    if backend == BackendPreference::DriverKitFirst {
        if let Some(health) = driverkit_health {
            if health.quarantined_count > 0 {
                supported.retain(|kind| {
                    *kind != HybridFeatureKind::Hotplug && *kind != HybridFeatureKind::PowerManagement
                });
            }
            if health.dispatch_failure_count > health.dispatch_success_count.saturating_add(2) {
                supported.retain(|kind| *kind != HybridFeatureKind::Snapshot);
            }
        }
    }

    supported
}

pub(crate) fn synthetic_coverage_request(kind: HybridRequestKind, index: usize) -> HybridRequest {
    let mmio_base = 0x1000 + (index * 0x200);
    let iova_base = 0x8000 + (index * 0x400);
    let irq_vector = 32 + (index as u32);

    match kind {
        HybridRequestKind::WindowsPe => HybridRequest::windows_pe(),
        HybridRequestKind::UserModeDevice => {
            HybridRequest::user_mode_device(mmio_base, 0x1000, irq_vector)
        }
        _ => HybridRequest::device(kind, mmio_base, 0x100, iova_base, 0x1000, irq_vector),
    }
}

pub(crate) fn request_family(kind: HybridRequestKind) -> HybridRequestFamily {
    match kind {
        HybridRequestKind::Network
        | HybridRequestKind::Ethernet
        | HybridRequestKind::Modem
        | HybridRequestKind::WiFi
        | HybridRequestKind::Bluetooth
        | HybridRequestKind::Nfc
        | HybridRequestKind::Can => HybridRequestFamily::Network,
        HybridRequestKind::Block | HybridRequestKind::Storage | HybridRequestKind::Nvme => {
            HybridRequestFamily::Storage
        }
        HybridRequestKind::Gpu | HybridRequestKind::Camera | HybridRequestKind::Audio => {
            HybridRequestFamily::Multimedia
        }
        HybridRequestKind::Sensor | HybridRequestKind::SensorHub | HybridRequestKind::Rtc => {
            HybridRequestFamily::Platform
        }
        HybridRequestKind::Input | HybridRequestKind::Touch | HybridRequestKind::Gamepad => {
            HybridRequestFamily::Input
        }
        HybridRequestKind::Tpm | HybridRequestKind::SmartCard => HybridRequestFamily::Security,
        HybridRequestKind::WindowsPe => HybridRequestFamily::Compatibility,
        HybridRequestKind::UserModeDevice => HybridRequestFamily::Platform,
        HybridRequestKind::Display | HybridRequestKind::Usb | HybridRequestKind::Dock | HybridRequestKind::Serial => {
            HybridRequestFamily::Peripheral
        }
        HybridRequestKind::Printer | HybridRequestKind::Firmware => HybridRequestFamily::Compatibility,
    }
}

pub(crate) fn required_features_for_request(kind: HybridRequestKind) -> Vec<HybridFeatureKind> {
    match request_family(kind) {
        HybridRequestFamily::Network => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Dma,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
        ],
        HybridRequestFamily::Storage => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Dma,
            HybridFeatureKind::Irq,
            HybridFeatureKind::Reset,
        ],
        HybridRequestFamily::Multimedia => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Dma,
            HybridFeatureKind::Irq,
            HybridFeatureKind::SharedMemory,
            HybridFeatureKind::ControlQueue,
        ],
        HybridRequestFamily::Input => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Hotplug,
        ],
        HybridRequestFamily::Security => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
            HybridFeatureKind::PowerManagement,
        ],
        HybridRequestFamily::Platform => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
            HybridFeatureKind::PowerManagement,
        ],
        HybridRequestFamily::Compatibility => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
            HybridFeatureKind::Snapshot,
        ],
        HybridRequestFamily::Peripheral => vec![
            HybridFeatureKind::Mmio,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Hotplug,
            HybridFeatureKind::PowerManagement,
        ],
    }
}

pub(crate) fn mandatory_features_for_request(kind: HybridRequestKind) -> Vec<HybridFeatureKind> {
    match request_family(kind) {
        HybridRequestFamily::Network => vec![
            HybridFeatureKind::Dma,
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
        ],
        HybridRequestFamily::Storage => vec![
            HybridFeatureKind::Dma,
            HybridFeatureKind::Irq,
            HybridFeatureKind::Reset,
        ],
        HybridRequestFamily::Multimedia => vec![HybridFeatureKind::Dma, HybridFeatureKind::Irq],
        HybridRequestFamily::Input => vec![
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
        ],
        HybridRequestFamily::Security => vec![
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Reset,
        ],
        HybridRequestFamily::Platform => vec![
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
        ],
        HybridRequestFamily::Compatibility => vec![
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Snapshot,
        ],
        HybridRequestFamily::Peripheral => vec![
            HybridFeatureKind::Irq,
            HybridFeatureKind::ControlQueue,
            HybridFeatureKind::Hotplug,
        ],
    }
}

pub(crate) fn required_coverage_ratio_for_request(kind: HybridRequestKind) -> (usize, usize) {
    match request_family(kind) {
        HybridRequestFamily::Storage | HybridRequestFamily::Security | HybridRequestFamily::Compatibility => {
            (3, 4)
        }
        _ => (2, 3),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FeatureCoverageStats {
    pub supported: Vec<HybridFeatureKind>,
    pub required_total: usize,
    pub required_covered: usize,
    pub mandatory_total: usize,
    pub mandatory_covered: usize,
    pub required_ratio_num: usize,
    pub required_ratio_den: usize,
}

impl FeatureCoverageStats {
    pub(crate) fn required_ready(&self) -> bool {
        self.required_total == 0
            || self.required_covered * self.required_ratio_den
                >= self.required_total * self.required_ratio_num
    }

    pub(crate) fn mandatory_ready(&self) -> bool {
        self.mandatory_covered == self.mandatory_total
    }

    pub(crate) fn missing_required(&self) -> usize {
        self.required_total.saturating_sub(self.required_covered)
    }

    pub(crate) fn missing_mandatory(&self) -> usize {
        self.mandatory_total.saturating_sub(self.mandatory_covered)
    }
}

pub(crate) fn feature_coverage_stats_for(
    request_kind: HybridRequestKind,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> FeatureCoverageStats {
    let supported = supported_features_for(request_kind, backend, driverkit_health);
    let required = required_features_for_request(request_kind);
    let mandatory = mandatory_features_for_request(request_kind);
    let (required_ratio_num, required_ratio_den) = required_coverage_ratio_for_request(request_kind);

    let required_covered = required
        .iter()
        .filter(|feature| supported.contains(feature))
        .count();
    let mandatory_covered = mandatory
        .iter()
        .filter(|feature| supported.contains(feature))
        .count();

    FeatureCoverageStats {
        supported,
        required_total: required.len(),
        required_covered,
        mandatory_total: mandatory.len(),
        mandatory_covered,
        required_ratio_num,
        required_ratio_den,
    }
}