use crate::modules::drivers::hybrid::orchestrator::{HybridRequest, HybridRequestKind};
use super::super::request_family;

#[derive(Clone, Copy)]
pub struct RequestShape {
    pub dma_intensity: i16,
    pub latency_sensitivity: i16,
    pub security_sensitivity: i16,
    pub compatibility_pressure: i16,
    pub bandwidth_pressure: i16,
    pub control_plane_pressure: i16,
}

pub fn merge_shapes(base: RequestShape, delta: RequestShape) -> RequestShape {
    RequestShape {
        dma_intensity: (base.dma_intensity + delta.dma_intensity).clamp(1, 24),
        latency_sensitivity: (base.latency_sensitivity + delta.latency_sensitivity).clamp(1, 24),
        security_sensitivity: (base.security_sensitivity + delta.security_sensitivity).clamp(1, 24),
        compatibility_pressure: (base.compatibility_pressure + delta.compatibility_pressure).clamp(1, 24),
        bandwidth_pressure: (base.bandwidth_pressure + delta.bandwidth_pressure).clamp(1, 24),
        control_plane_pressure: (base.control_plane_pressure + delta.control_plane_pressure).clamp(1, 24),
    }
}

pub fn request_shape(kind: HybridRequestKind) -> RequestShape {
    match kind {
        HybridRequestKind::Network
        | HybridRequestKind::Ethernet
        | HybridRequestKind::WiFi
        | HybridRequestKind::Bluetooth
        | HybridRequestKind::Modem
        | HybridRequestKind::Nfc
        | HybridRequestKind::Can => RequestShape {
            dma_intensity: 6,
            latency_sensitivity: 8,
            security_sensitivity: 6,
            compatibility_pressure: 4,
            bandwidth_pressure: 12,
            control_plane_pressure: 6,
        },
        HybridRequestKind::Block | HybridRequestKind::Storage | HybridRequestKind::Nvme => RequestShape {
            dma_intensity: 14,
            latency_sensitivity: 10,
            security_sensitivity: 4,
            compatibility_pressure: 4,
            bandwidth_pressure: 10,
            control_plane_pressure: 4,
        },
        HybridRequestKind::Gpu | HybridRequestKind::Camera | HybridRequestKind::Audio | HybridRequestKind::Display => RequestShape {
            dma_intensity: 8,
            latency_sensitivity: 12,
            security_sensitivity: 4,
            compatibility_pressure: 6,
            bandwidth_pressure: 14,
            control_plane_pressure: 4,
        },
        HybridRequestKind::Input | HybridRequestKind::Touch | HybridRequestKind::Gamepad => RequestShape {
            dma_intensity: 4,
            latency_sensitivity: 14,
            security_sensitivity: 5,
            compatibility_pressure: 6,
            bandwidth_pressure: 4,
            control_plane_pressure: 10,
        },
        HybridRequestKind::Tpm | HybridRequestKind::SmartCard => RequestShape {
            dma_intensity: 2,
            latency_sensitivity: 6,
            security_sensitivity: 18,
            compatibility_pressure: 8,
            bandwidth_pressure: 2,
            control_plane_pressure: 12,
        },
        HybridRequestKind::WindowsPe => RequestShape {
            dma_intensity: 2,
            latency_sensitivity: 4,
            security_sensitivity: 6,
            compatibility_pressure: 20,
            bandwidth_pressure: 2,
            control_plane_pressure: 10,
        },
        HybridRequestKind::UserModeDevice => RequestShape {
            dma_intensity: 6,
            latency_sensitivity: 10,
            security_sensitivity: 8,
            compatibility_pressure: 6,
            bandwidth_pressure: 4,
            control_plane_pressure: 16,
        },
        HybridRequestKind::Rtc | HybridRequestKind::SensorHub | HybridRequestKind::Sensor => RequestShape {
            dma_intensity: 4,
            latency_sensitivity: 12,
            security_sensitivity: 6,
            compatibility_pressure: 4,
            bandwidth_pressure: 4,
            control_plane_pressure: 10,
        },
        _ => RequestShape {
            dma_intensity: 6,
            latency_sensitivity: 6,
            security_sensitivity: 4,
            compatibility_pressure: 10,
            bandwidth_pressure: 6,
            control_plane_pressure: 10,
        },
    }
}

pub fn request_resource_shape(request: &HybridRequest) -> RequestShape {
    let mmio_kib = (request.mmio_length / 1024) as i16;
    let iova_kib = (request.iova_length / 1024) as i16;
    let mmio_pressure = (mmio_kib / 8).clamp(0, 8);
    let dma_pressure = (iova_kib / 32).clamp(0, 10);
    let irq_pressure = if request.irq_vector >= 128 {
        2
    } else if request.irq_vector >= 64 {
        1
    } else {
        0
    };

    RequestShape {
        dma_intensity: dma_pressure,
        latency_sensitivity: irq_pressure,
        security_sensitivity: if request.kind == HybridRequestKind::UserModeDevice { 2 } else { 0 },
        compatibility_pressure: mmio_pressure / 2,
        bandwidth_pressure: mmio_pressure + dma_pressure / 2,
        control_plane_pressure: irq_pressure + mmio_pressure / 2,
    }
}

pub fn observed_request_shape(request: &HybridRequest) -> RequestShape {
    let base = request_shape(request.kind);
    let family = request_family(request.kind);
    let (family_shape, _) = super::family_demand(family);
    let family_delta = RequestShape {
        dma_intensity: (family_shape.dma_intensity - base.dma_intensity) / 3,
        latency_sensitivity: (family_shape.latency_sensitivity - base.latency_sensitivity) / 3,
        security_sensitivity: (family_shape.security_sensitivity - base.security_sensitivity) / 3,
        compatibility_pressure: (family_shape.compatibility_pressure - base.compatibility_pressure) / 3,
        bandwidth_pressure: (family_shape.bandwidth_pressure - base.bandwidth_pressure) / 3,
        control_plane_pressure: (family_shape.control_plane_pressure - base.control_plane_pressure) / 3,
    };

    let resource_delta = request_resource_shape(request);
    merge_shapes(base, merge_shapes(family_delta, resource_delta))
}
