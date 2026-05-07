use super::super::super::driverkit::UserModeDriverContext;
use super::super::super::linux::LinuxResourcePlan;
use super::super::super::reactos::{
    NtExecutionPolicy, PeImageInfo, NtImportBinding, NtDomainImportBinding, NtImportDomainCounts
};
use super::super::super::sidecar::{SideCarPayload, SideCarVmPlan, SideCarWireHeader};
use super::BackendPreference;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HybridExecutionPlan {
    SideCar(SideCarVmPlan),
    LibLinux(LinuxResourcePlan),
    ReactOs {
        policy: NtExecutionPolicy,
        image_info: PeImageInfo,
    },
    DriverKit(UserModeDriverContext),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCarWireFrame {
    pub header: SideCarWireHeader,
    pub payload: SideCarPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridPlanAttempt {
    pub backend: BackendPreference,
    pub matched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridPlanDiagnostics {
    pub attempts: Vec<HybridPlanAttempt>,
    pub selected: Option<HybridExecutionPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactOsImportResolution {
    pub image_info: PeImageInfo,
    pub bindings: Vec<NtImportBinding>,
    pub domain_bindings: Vec<NtDomainImportBinding>,
    pub counts: NtImportDomainCounts,
    pub policy: NtExecutionPolicy,
}
