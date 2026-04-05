use alloc::vec::Vec;
use core::fmt;

use super::lifecycle::DriverErrorKind;

pub mod linux;
pub mod sidecar;
pub mod liblinux;
pub mod reactos;
pub mod driverkit;
pub mod orchestrator;

pub use driverkit::{
    DriverBindingRecord, DriverKitClass, DriverKitEvent, DriverKitEventQueue,
    DriverKitHealthSnapshot, DriverKitRecoveryPolicy, DriverKitRegistry, DriverLifecycleState,
    UserModeDriverContext,
};
pub use liblinux::{
    LibLinuxBackendKind, LibLinuxBridge, LinuxSyscall, LinuxSyscallDispatcher,
    LinuxBridgeDispatchRecord, LinuxSyscallMapper, LinuxSyscallQueue, LinuxSyscallRequest,
    LinuxSyscallResponse, LibLinuxConformanceReport, LibLinuxConformanceRisk,
    LibLinuxDispatchSample, LibLinuxTelemetryStore, LibLinuxTelemetrySummary,
    ZeroCopyIoPolicy,
};
pub use reactos::{
    Irql, NtBinaryExecutionMode, NtExecutionPolicy, NtIrqlGuard, NtSpinLock, NtSymbol,
    NtDomainImportBinding, NtImportBinding, NtImportDomain, NtImportDomainCounts,
    NtImportResolutionReport, NtSymbolTable, PeImageInfo, PeImportDescriptor, PeImportName,
    PeLoadError, PeRelocationBlock, PeSectionInfo, RelocationPatch,
};
pub use sidecar::{
    InMemorySideCarTransport, SideCarInterruptRoute, SideCarOpcode, SideCarPayload,
    SideCarBootstrapPhase, SideCarBootstrapState, SideCarBootstrapSummary, SideCarPayloadTag,
    SideCarQueueClass, SideCarRetryPolicy, SideCarTransport, SideCarVmConfig, SideCarVmPlan,
    SideCarTelemetrySample, SideCarTelemetrySnapshot, SideCarTelemetrySnapshotBucket,
    SideCarTelemetryStore, SideCarTelemetrySummary, SideCarSaturationLevel,
    SideCarWireError, SideCarWireHeader, VirtioQueueSelector,
};
pub use orchestrator::{
    BackendPreference, HybridExecutionPlan, HybridOrchestrator, HybridOrchestratorSession,
    HybridRequest,
    HybridRequestKind, HybridBackendSupport, HybridSupportReport, HybridCoverageAudit,
    HybridCoverageRow, HybridFeatureAudit, HybridFeatureKind, HybridFeatureRow,
    HybridGapSeverity, HybridReadinessGap, HybridReadinessReport,
    ReactOsImportResolution, SideCarWireFrame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverTransportKind {
    SideCarVm,
    LibLinux,
    ReactOs,
    DriverKit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DriverCapabilitySet(u64);

impl DriverCapabilitySet {
    pub const MMIO: Self = Self(1 << 0);
    pub const DMA: Self = Self(1 << 1);
    pub const IRQ: Self = Self(1 << 2);
    pub const SHARED_MEMORY: Self = Self(1 << 3);
    pub const CONTROL_QUEUE: Self = Self(1 << 4);
    pub const RESET: Self = Self(1 << 5);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn all() -> Self {
        Self(
            Self::MMIO.0
                | Self::DMA.0
                | Self::IRQ.0
                | Self::SHARED_MEMORY.0
                | Self::CONTROL_QUEUE.0
                | Self::RESET.0,
        )
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn with(self, other: Self) -> Self {
        self.union(other)
    }
}

impl core::ops::BitOr for DriverCapabilitySet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for DriverCapabilitySet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for DriverCapabilitySet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmioGrant {
    pub base: usize,
    pub length: usize,
    pub writable: bool,
    pub cached: bool,
}

impl MmioGrant {
    pub const fn new(base: usize, length: usize) -> Self {
        Self {
            base,
            length,
            writable: true,
            cached: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaGrant {
    pub iova_base: usize,
    pub length: usize,
    pub read: bool,
    pub write: bool,
    pub coherent: bool,
}

impl DmaGrant {
    pub const fn new(iova_base: usize, length: usize) -> Self {
        Self {
            iova_base,
            length,
            read: true,
            write: true,
            coherent: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrqGrant {
    pub vector: u32,
    pub msi_index: Option<u16>,
    pub direct_injection: bool,
}

impl IrqGrant {
    pub const fn new(vector: u32) -> Self {
        Self {
            vector,
            msi_index: None,
            direct_injection: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedMemoryGrant {
    pub base: usize,
    pub length: usize,
    pub writable: bool,
}

impl SharedMemoryGrant {
    pub const fn new(base: usize, length: usize) -> Self {
        Self {
            base,
            length,
            writable: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverResources {
    pub transport: DriverTransportKind,
    pub capabilities: DriverCapabilitySet,
    pub mmio: Vec<MmioGrant>,
    pub dma: Vec<DmaGrant>,
    pub irqs: Vec<IrqGrant>,
    pub shared_memory: Vec<SharedMemoryGrant>,
}

impl DriverResources {
    pub fn new(transport: DriverTransportKind) -> Self {
        Self {
            transport,
            capabilities: DriverCapabilitySet::empty(),
            mmio: Vec::new(),
            dma: Vec::new(),
            irqs: Vec::new(),
            shared_memory: Vec::new(),
        }
    }

    pub fn with_capabilities(mut self, capabilities: DriverCapabilitySet) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn add_mmio(mut self, grant: MmioGrant) -> Self {
        self.mmio.push(grant);
        self
    }

    pub fn add_dma(mut self, grant: DmaGrant) -> Self {
        self.dma.push(grant);
        self
    }

    pub fn add_irq(mut self, grant: IrqGrant) -> Self {
        self.irqs.push(grant);
        self
    }

    pub fn add_shared_memory(mut self, grant: SharedMemoryGrant) -> Self {
        self.shared_memory.push(grant);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDataPlaneHint {
    ControlOnly,
    SharedMemoryPreferred,
    SharedMemoryRequired,
    PinnedPagesOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedBufferDescriptor {
    pub offset: usize,
    pub length: usize,
}

impl SharedBufferDescriptor {
    pub const fn new(offset: usize, length: usize) -> Self {
        Self { offset, length }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScatterGatherList {
    pub segments: Vec<SharedBufferDescriptor>,
}

impl ScatterGatherList {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn from_segments(segments: Vec<SharedBufferDescriptor>) -> Self {
        Self { segments }
    }

    pub fn push(&mut self, segment: SharedBufferDescriptor) {
        self.segments.push(segment);
    }

    pub fn total_length(&self) -> usize {
        self.segments.iter().map(|segment| segment.length).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl Default for ScatterGatherList {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxIoRequestKind {
    NetRx,
    NetTx,
    BlockRead,
    BlockWrite,
    Control,
    InterruptAck,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxIoRequest {
    pub request_id: u64,
    pub kind: LinuxIoRequestKind,
    pub data_plane_hint: LinuxDataPlaneHint,
    pub payload: ScatterGatherList,
}

impl LinuxIoRequest {
    pub fn new(request_id: u64, kind: LinuxIoRequestKind) -> Self {
        Self {
            request_id,
            kind,
            data_plane_hint: LinuxDataPlaneHint::ControlOnly,
            payload: ScatterGatherList::new(),
        }
    }

    pub fn with_payload(mut self, payload: ScatterGatherList) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_hint(mut self, hint: LinuxDataPlaneHint) -> Self {
        self.data_plane_hint = hint;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverCompletion {
    pub request_id: u64,
    pub status: Result<(), DriverErrorKind>,
    pub bytes_transferred: usize,
}

impl DriverCompletion {
    pub const fn ok(request_id: u64, bytes_transferred: usize) -> Self {
        Self {
            request_id,
            status: Ok(()),
            bytes_transferred,
        }
    }

    pub const fn err(request_id: u64, error: DriverErrorKind) -> Self {
        Self {
            request_id,
            status: Err(error),
            bytes_transferred: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverOp {
    Control(LinuxIoRequest),
    Data(LinuxIoRequest),
    Reset,
    QueryCaps,
}

pub trait DriverProvider {
    type Device;
    type Handle;
    type Error;

    fn probe(&self, device: &Self::Device) -> Result<bool, Self::Error>;
    fn attach(
        &mut self,
        device: Self::Device,
        resources: DriverResources,
    ) -> Result<Self::Handle, Self::Error>;
    fn detach(&mut self, handle: Self::Handle) -> Result<(), Self::Error>;
    fn suspend(&mut self, handle: &mut Self::Handle) -> Result<(), Self::Error>;
    fn resume(&mut self, handle: &mut Self::Handle) -> Result<(), Self::Error>;
    fn submit(
        &mut self,
        handle: &mut Self::Handle,
        op: DriverOp,
    ) -> Result<DriverCompletion, Self::Error>;
    fn poll(&mut self, handle: &mut Self::Handle) -> Result<Option<DriverCompletion>, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxBridgeMessageKind {
    Discover,
    Attach,
    Detach,
    MapDma,
    UnmapDma,
    NotifyQueue,
    InterruptAck,
    Reset,
    QueryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxBridgeHeader {
    pub version: u16,
    pub kind: LinuxBridgeMessageKind,
    pub flags: u16,
    pub request_id: u64,
    pub device_id: u32,
    pub payload_len: u32,
}

impl LinuxBridgeHeader {
    pub const VERSION: u16 = 1;

    pub const fn new(kind: LinuxBridgeMessageKind, request_id: u64, payload_len: u32) -> Self {
        Self {
            version: Self::VERSION,
            kind,
            flags: 0,
            request_id,
            device_id: 0,
            payload_len,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxBridgePayload {
    Empty,
    Resources(DriverResources),
    Request(LinuxIoRequest),
    Completion(DriverCompletion),
    Error(DriverErrorKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxBridgeMessage {
    pub header: LinuxBridgeHeader,
    pub payload: LinuxBridgePayload,
}

impl LinuxBridgeMessage {
    pub fn new(kind: LinuxBridgeMessageKind, request_id: u64, payload: LinuxBridgePayload) -> Self {
        let payload_len = match &payload {
            LinuxBridgePayload::Empty => 0,
            LinuxBridgePayload::Resources(resources) => {
                (resources.mmio.len()
                    + resources.dma.len()
                    + resources.irqs.len()
                    + resources.shared_memory.len()) as u32
            }
            LinuxBridgePayload::Request(request) => request.payload.total_length() as u32,
            LinuxBridgePayload::Completion(completion) => completion.bytes_transferred as u32,
            LinuxBridgePayload::Error(_) => 0,
        };

        Self {
            header: LinuxBridgeHeader::new(kind, request_id, payload_len),
            payload,
        }
    }
}

impl fmt::Display for DriverTransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            DriverTransportKind::SideCarVm => "sidecar-vm",
            DriverTransportKind::LibLinux => "liblinux",
            DriverTransportKind::ReactOs => "reactos",
            DriverTransportKind::DriverKit => "driverkit",
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn capability_set_combines_flags() {
        let caps = DriverCapabilitySet::MMIO | DriverCapabilitySet::DMA;
        assert!(caps.contains(DriverCapabilitySet::MMIO));
        assert!(caps.contains(DriverCapabilitySet::DMA));
        assert!(!caps.contains(DriverCapabilitySet::IRQ));
    }

    #[test_case]
    fn scatter_gather_total_length_counts_segments() {
        let mut list = ScatterGatherList::new();
        list.push(SharedBufferDescriptor::new(0, 128));
        list.push(SharedBufferDescriptor::new(128, 64));

        assert_eq!(list.total_length(), 192);
        assert!(!list.is_empty());
    }

    #[test_case]
    fn linux_bridge_message_computes_payload_length() {
        let request = LinuxIoRequest::new(7, LinuxIoRequestKind::NetTx)
            .with_hint(LinuxDataPlaneHint::SharedMemoryPreferred)
            .with_payload(ScatterGatherList::from_segments(vec![
                SharedBufferDescriptor::new(0, 64),
                SharedBufferDescriptor::new(64, 128),
            ]));

        let message = LinuxBridgeMessage::new(
            LinuxBridgeMessageKind::NotifyQueue,
            7,
            LinuxBridgePayload::Request(request),
        );

        assert_eq!(message.header.version, LinuxBridgeHeader::VERSION);
        assert_eq!(message.header.payload_len, 192);
    }
}