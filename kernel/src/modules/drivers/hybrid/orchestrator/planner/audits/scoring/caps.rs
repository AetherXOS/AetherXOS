use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::{BackendPreference, HybridFeatureKind, HybridRequestKind};
use super::super::feature_coverage_stats_for;

#[derive(Clone, Copy)]
pub struct BackendCapabilities {
    pub isolation: i16,
    pub native_semantics: i16,
    pub mediation: i16,
    pub compatibility: i16,
    pub throughput: i16,
    pub security_containment: i16,
}

pub fn backend_capabilities(backend: BackendPreference) -> BackendCapabilities {
    match backend {
        BackendPreference::SideCarFirst => BackendCapabilities {
            isolation: 18,
            native_semantics: 8,
            mediation: 12,
            compatibility: 4,
            throughput: 10,
            security_containment: 16,
        },
        BackendPreference::LibLinuxFirst => BackendCapabilities {
            isolation: 6,
            native_semantics: 16,
            mediation: 8,
            compatibility: 8,
            throughput: 16,
            security_containment: 8,
        },
        BackendPreference::DriverKitFirst => BackendCapabilities {
            isolation: 12,
            native_semantics: 6,
            mediation: 14,
            compatibility: 6,
            throughput: 4,
            security_containment: 12,
        },
        BackendPreference::ReactOsFirst => BackendCapabilities {
            isolation: 4,
            native_semantics: 4,
            mediation: 6,
            compatibility: 20,
            throughput: 2,
            security_containment: 2,
        },
    }
}

pub fn feature_capability_delta(
    request_kind: HybridRequestKind,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> BackendCapabilities {
    let coverage = feature_coverage_stats_for(request_kind, backend, driverkit_health);
    let supported = &coverage.supported;
    let mut delta = BackendCapabilities {
        isolation: 0,
        native_semantics: 0,
        mediation: 0,
        compatibility: 0,
        throughput: 0,
        security_containment: 0,
    };

    for feature in supported {
        match feature {
            HybridFeatureKind::Mmio => {
                delta.native_semantics += 2;
                delta.compatibility += 1;
            }
            HybridFeatureKind::Dma => {
                delta.throughput += 4;
                delta.native_semantics += 1;
            }
            HybridFeatureKind::Irq => {
                delta.mediation += 2;
                delta.throughput += 1;
            }
            HybridFeatureKind::SharedMemory => {
                delta.isolation += 2;
                delta.mediation += 1;
                delta.throughput += 2;
            }
            HybridFeatureKind::ControlQueue => {
                delta.mediation += 3;
                delta.compatibility += 1;
            }
            HybridFeatureKind::Reset => {
                delta.mediation += 1;
            }
            HybridFeatureKind::Hotplug => {
                delta.compatibility += 2;
                delta.mediation += 1;
            }
            HybridFeatureKind::PowerManagement => {
                delta.mediation += 1;
                delta.security_containment += 1;
            }
            HybridFeatureKind::Snapshot => {
                delta.isolation += 1;
                delta.compatibility += 1;
            }
            HybridFeatureKind::LiveMigration => {
                delta.compatibility += 3;
                delta.mediation += 2;
                delta.throughput += 1;
            }
        }
    }

    let missing = 10i16.saturating_sub(supported.len() as i16);
    delta.throughput -= missing / 2;
    delta.compatibility -= missing / 3;
    delta.security_containment -= missing / 4;

    let missing_required = coverage.missing_required() as i16;
    let missing_mandatory = coverage.missing_mandatory() as i16;
    delta.compatibility -= missing_required * 2;
    delta.mediation -= missing_required * 2;
    delta.security_containment -= missing_required;
    delta.compatibility -= missing_mandatory * 3;
    delta.mediation -= missing_mandatory * 3;
    delta.throughput -= missing_mandatory * 2;
    delta.security_containment -= missing_mandatory * 2;
    delta
}

pub fn observed_backend_capabilities(
    request_kind: HybridRequestKind,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> BackendCapabilities {
    let base = backend_capabilities(backend);
    let delta = feature_capability_delta(request_kind, backend, driverkit_health);
    BackendCapabilities {
        isolation: (base.isolation + delta.isolation).clamp(1, 40),
        native_semantics: (base.native_semantics + delta.native_semantics).clamp(1, 40),
        mediation: (base.mediation + delta.mediation).clamp(1, 40),
        compatibility: (base.compatibility + delta.compatibility).clamp(1, 40),
        throughput: (base.throughput + delta.throughput).clamp(1, 40),
        security_containment: (base.security_containment + delta.security_containment).clamp(1, 40),
    }
}
