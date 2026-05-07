pub mod caps;
pub mod shapes;
pub mod logic;
pub mod classification;
pub mod metrics;

pub use logic::*;
pub use classification::*;
pub use metrics::*;

use crate::modules::drivers::hybrid::orchestrator::{
    BackendPreference, HybridRequestFamily,
};

pub fn family_demand(family: HybridRequestFamily) -> (shapes::RequestShape, &'static str) {
    match family {
        HybridRequestFamily::Network => (
            shapes::RequestShape {
                dma_intensity: 4,
                latency_sensitivity: 12,
                security_sensitivity: 8,
                compatibility_pressure: 4,
                bandwidth_pressure: 16,
                control_plane_pressure: 8,
            },
            "network family demands throughput, queue ordering, and stable mediation",
        ),
        HybridRequestFamily::Storage => (
            shapes::RequestShape {
                dma_intensity: 16,
                latency_sensitivity: 10,
                security_sensitivity: 6,
                compatibility_pressure: 4,
                bandwidth_pressure: 14,
                control_plane_pressure: 4,
            },
            "storage family demands DMA discipline and high request throughput",
        ),
        HybridRequestFamily::Multimedia => (
            shapes::RequestShape {
                dma_intensity: 8,
                latency_sensitivity: 14,
                security_sensitivity: 4,
                compatibility_pressure: 6,
                bandwidth_pressure: 16,
                control_plane_pressure: 4,
            },
            "multimedia family demands low jitter and high bandwidth fan-out",
        ),
        HybridRequestFamily::Input => (
            shapes::RequestShape {
                dma_intensity: 4,
                latency_sensitivity: 16,
                security_sensitivity: 6,
                compatibility_pressure: 6,
                bandwidth_pressure: 4,
                control_plane_pressure: 10,
            },
            "input family demands low latency and precise event handling",
        ),
        HybridRequestFamily::Security => (
            shapes::RequestShape {
                dma_intensity: 2,
                latency_sensitivity: 8,
                security_sensitivity: 18,
                compatibility_pressure: 8,
                bandwidth_pressure: 2,
                control_plane_pressure: 14,
            },
            "security family demands strong containment and controlled mediation",
        ),
        HybridRequestFamily::Platform => (
            shapes::RequestShape {
                dma_intensity: 4,
                latency_sensitivity: 8,
                security_sensitivity: 10,
                compatibility_pressure: 10,
                bandwidth_pressure: 6,
                control_plane_pressure: 16,
            },
            "platform family demands management reach and wake coordination",
        ),
        HybridRequestFamily::Compatibility => (
            shapes::RequestShape {
                dma_intensity: 2,
                latency_sensitivity: 6,
                security_sensitivity: 4,
                compatibility_pressure: 20,
                bandwidth_pressure: 2,
                control_plane_pressure: 12,
            },
            "compatibility family demands ABI fidelity over raw throughput",
        ),
        HybridRequestFamily::Peripheral => (
            shapes::RequestShape {
                dma_intensity: 6,
                latency_sensitivity: 8,
                security_sensitivity: 6,
                compatibility_pressure: 10,
                bandwidth_pressure: 8,
                control_plane_pressure: 10,
            },
            "peripheral family demands hotplug stability and control-path balance",
        ),
    }
}

pub fn family_reason(_backend: BackendPreference, family: HybridRequestFamily) -> &'static str {
    let (_demand, reason) = family_demand(family);
    reason
}