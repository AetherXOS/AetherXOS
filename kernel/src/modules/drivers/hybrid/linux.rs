use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::{
    DmaGrant, DriverCapabilitySet, DriverResources, DriverTransportKind, IrqGrant,
    LinuxBridgeMessage, LinuxBridgeMessageKind, LinuxBridgePayload, LinuxDataPlaneHint,
    LinuxIoRequest, LinuxIoRequestKind, MmioGrant, ScatterGatherList, SharedBufferDescriptor,
    SharedMemoryGrant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxShimDeviceKind {
    Network,
    Block,
    Ethernet,
    Storage,
    Modem,
    Printer,
    Rtc,
    SensorHub,
    Gpu,
    WiFi,
    Bluetooth,
    Nfc,
    Tpm,
    Dock,
    Display,
    Usb,
    Can,
    Serial,
    Firmware,
    SmartCard,
    Nvme,
    Touch,
    Gamepad,
    Camera,
    Audio,
    Sensor,
    Input,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxZeroCopyHint {
    None,
    ReadOnlyGrant,
    SharedTxRxPages,
    PinnedScatterGather,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxBridgeChannel {
    Control,
    Completion,
    Tx,
    Rx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxBridgeQueue {
    pub channel: LinuxBridgeChannel,
    pub depth: usize,
    frames: VecDeque<LinuxBridgeMessage>,
}

impl LinuxBridgeQueue {
    pub fn new(channel: LinuxBridgeChannel, depth: usize) -> Self {
        Self {
            channel,
            depth: depth.max(1),
            frames: VecDeque::new(),
        }
    }

    pub fn push(&mut self, message: LinuxBridgeMessage) -> Result<(), LinuxBridgeMessage> {
        if self.frames.len() >= self.depth {
            return Err(message);
        }
        self.frames.push_back(message);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<LinuxBridgeMessage> {
        self.frames.pop_front()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.frames.len() >= self.depth
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxMemoryWindow {
    pub grant: SharedMemoryGrant,
    pub zero_copy: LinuxZeroCopyHint,
    pub cache_coherent: bool,
}

impl LinuxMemoryWindow {
    pub const fn new(grant: SharedMemoryGrant, zero_copy: LinuxZeroCopyHint) -> Self {
        Self {
            grant,
            zero_copy,
            cache_coherent: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxPageGrant {
    pub grant: DmaGrant,
    pub pinned_pages: usize,
}

impl LinuxPageGrant {
    pub const fn new(grant: DmaGrant, pinned_pages: usize) -> Self {
        let pinned_pages = if pinned_pages == 0 { 1 } else { pinned_pages };
        Self {
            grant,
            pinned_pages,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxResourcePlan {
    pub transport: DriverTransportKind,
    pub device_kind: LinuxShimDeviceKind,
    pub resources: DriverResources,
    pub control_queue_depth: usize,
    pub completion_queue_depth: usize,
    pub data_queue_depth: usize,
    pub memory_windows: Vec<LinuxMemoryWindow>,
    pub page_grants: Vec<LinuxPageGrant>,
    pub zero_copy_hint: LinuxZeroCopyHint,
    pub irq_coalescing_budget: usize,
}

impl LinuxResourcePlan {
    pub fn new(transport: DriverTransportKind, device_kind: LinuxShimDeviceKind) -> Self {
        let resources = DriverResources::new(transport)
            .with_capabilities(DriverCapabilitySet::MMIO | DriverCapabilitySet::DMA);

        Self {
            transport,
            device_kind,
            resources,
            control_queue_depth: 32,
            completion_queue_depth: 32,
            data_queue_depth: 128,
            memory_windows: Vec::new(),
            page_grants: Vec::new(),
            zero_copy_hint: LinuxZeroCopyHint::PinnedScatterGather,
            irq_coalescing_budget: 4,
        }
    }

    pub fn with_resources(mut self, resources: DriverResources) -> Self {
        self.resources = resources;
        self
    }

    pub fn with_queue_depths(
        mut self,
        control_queue_depth: usize,
        completion_queue_depth: usize,
        data_queue_depth: usize,
    ) -> Self {
        self.control_queue_depth = control_queue_depth.max(1);
        self.completion_queue_depth = completion_queue_depth.max(1);
        self.data_queue_depth = data_queue_depth.max(1);
        self
    }

    pub fn with_zero_copy_hint(mut self, hint: LinuxZeroCopyHint) -> Self {
        self.zero_copy_hint = hint;
        self
    }

    pub fn with_irq_coalescing_budget(mut self, budget: usize) -> Self {
        self.irq_coalescing_budget = budget.max(1);
        self
    }

    pub fn add_memory_window(mut self, window: LinuxMemoryWindow) -> Self {
        self.memory_windows.push(window);
        self
    }

    pub fn add_page_grant(mut self, page_grant: LinuxPageGrant) -> Self {
        self.page_grants.push(page_grant);
        self
    }
}

pub trait LinuxSyscallBridge {
    fn map_memory(&mut self, window: LinuxMemoryWindow) -> Result<(), LinuxBridgeMessage>;
    fn unmap_memory(&mut self, base: usize) -> Result<(), LinuxBridgeMessage>;
    fn map_dma(&mut self, grant: LinuxPageGrant) -> Result<(), LinuxBridgeMessage>;
    fn unmap_dma(&mut self, iova_base: usize) -> Result<(), LinuxBridgeMessage>;
    fn notify_irq(&mut self, irq: IrqGrant) -> Result<(), LinuxBridgeMessage>;
    fn submit_request(&mut self, request: LinuxIoRequest) -> Result<(), LinuxBridgeMessage>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxShimDriverContext {
    pub device_kind: LinuxShimDeviceKind,
    pub plan: LinuxResourcePlan,
    pub control: LinuxBridgeQueue,
    pub completion: LinuxBridgeQueue,
    pub tx: LinuxBridgeQueue,
    pub rx: LinuxBridgeQueue,
}

impl LinuxShimDriverContext {
    pub fn new(plan: LinuxResourcePlan) -> Self {
        let control_depth = plan.control_queue_depth;
        let completion_depth = plan.completion_queue_depth;
        let data_depth = plan.data_queue_depth;

        Self {
            device_kind: plan.device_kind,
            plan,
            control: LinuxBridgeQueue::new(LinuxBridgeChannel::Control, control_depth),
            completion: LinuxBridgeQueue::new(LinuxBridgeChannel::Completion, completion_depth),
            tx: LinuxBridgeQueue::new(LinuxBridgeChannel::Tx, data_depth),
            rx: LinuxBridgeQueue::new(LinuxBridgeChannel::Rx, data_depth),
        }
    }

    pub fn build_discovery_message(&self, request_id: u64) -> LinuxBridgeMessage {
        LinuxBridgeMessage::new(
            LinuxBridgeMessageKind::Discover,
            request_id,
            LinuxBridgePayload::Resources(self.plan.resources.clone()),
        )
    }

    pub fn build_queue_notify(&self, request_id: u64, request: LinuxIoRequest) -> LinuxBridgeMessage {
        LinuxBridgeMessage::new(
            LinuxBridgeMessageKind::NotifyQueue,
            request_id,
            LinuxBridgePayload::Request(request),
        )
    }

    pub fn build_reset(&self, request_id: u64) -> LinuxBridgeMessage {
        LinuxBridgeMessage::new(
            LinuxBridgeMessageKind::Reset,
            request_id,
            LinuxBridgePayload::Empty,
        )
    }

    pub fn enqueue_control(&mut self, message: LinuxBridgeMessage) -> Result<(), LinuxBridgeMessage> {
        self.control.push(message)
    }

    pub fn dequeue_completion(&mut self) -> Option<LinuxBridgeMessage> {
        self.completion.pop()
    }
}

pub fn build_network_plan(
    transport: DriverTransportKind,
    mmio_base: usize,
    mmio_length: usize,
    iova_base: usize,
    iova_length: usize,
    irq_vector: u32,
) -> LinuxResourcePlan {
    LinuxResourcePlan::new(transport, LinuxShimDeviceKind::Network)
        .with_queue_depths(64, 64, 256)
        .with_zero_copy_hint(LinuxZeroCopyHint::SharedTxRxPages)
        .add_memory_window(LinuxMemoryWindow::new(
            SharedMemoryGrant::new(mmio_base, mmio_length),
            LinuxZeroCopyHint::ReadOnlyGrant,
        ))
        .add_page_grant(LinuxPageGrant::new(DmaGrant::new(iova_base, iova_length), 1))
        .with_resources(
            DriverResources::new(transport)
                .with_capabilities(
                    DriverCapabilitySet::MMIO
                        | DriverCapabilitySet::DMA
                        | DriverCapabilitySet::IRQ
                        | DriverCapabilitySet::SHARED_MEMORY
                        | DriverCapabilitySet::CONTROL_QUEUE,
                )
                .add_mmio(MmioGrant::new(mmio_base, mmio_length))
                .add_dma(DmaGrant::new(iova_base, iova_length))
                .add_irq(IrqGrant::new(irq_vector)),
        )
}

pub fn build_block_plan(
    transport: DriverTransportKind,
    mmio_base: usize,
    mmio_length: usize,
    iova_base: usize,
    iova_length: usize,
    irq_vector: u32,
) -> LinuxResourcePlan {
    LinuxResourcePlan::new(transport, LinuxShimDeviceKind::Block)
        .with_queue_depths(16, 16, 64)
        .with_zero_copy_hint(LinuxZeroCopyHint::PinnedScatterGather)
        .add_memory_window(LinuxMemoryWindow::new(
            SharedMemoryGrant::new(mmio_base, mmio_length),
            LinuxZeroCopyHint::ReadOnlyGrant,
        ))
        .add_page_grant(LinuxPageGrant::new(DmaGrant::new(iova_base, iova_length), 1))
        .with_resources(
            DriverResources::new(transport)
                .with_capabilities(
                    DriverCapabilitySet::MMIO
                        | DriverCapabilitySet::DMA
                        | DriverCapabilitySet::IRQ
                        | DriverCapabilitySet::SHARED_MEMORY,
                )
                .add_mmio(MmioGrant::new(mmio_base, mmio_length))
                .add_dma(DmaGrant::new(iova_base, iova_length))
                .add_irq(IrqGrant::new(irq_vector)),
        )
}

pub fn make_data_request(
    request_id: u64,
    kind: LinuxIoRequestKind,
    zero_copy_hint: LinuxZeroCopyHint,
    segments: Vec<SharedBufferDescriptor>,
) -> LinuxIoRequest {
    LinuxIoRequest::new(request_id, kind)
        .with_hint(match zero_copy_hint {
            LinuxZeroCopyHint::None => LinuxDataPlaneHint::ControlOnly,
            LinuxZeroCopyHint::ReadOnlyGrant => LinuxDataPlaneHint::SharedMemoryPreferred,
            LinuxZeroCopyHint::SharedTxRxPages => LinuxDataPlaneHint::SharedMemoryRequired,
            LinuxZeroCopyHint::PinnedScatterGather => LinuxDataPlaneHint::PinnedPagesOnly,
        })
        .with_payload(ScatterGatherList::from_segments(segments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::drivers::hybrid::LinuxBridgeHeader;

    #[test_case]
    fn network_plan_populates_required_resources() {
        let plan = build_network_plan(DriverTransportKind::LibLinux, 0x1000, 0x100, 0x2000, 0x4000, 44);

        assert_eq!(plan.transport, DriverTransportKind::LibLinux);
        assert_eq!(plan.device_kind, LinuxShimDeviceKind::Network);
        assert_eq!(plan.resources.mmio.len(), 1);
        assert_eq!(plan.resources.dma.len(), 1);
        assert_eq!(plan.resources.irqs.len(), 1);
        assert!(plan.resources.capabilities.contains(DriverCapabilitySet::DMA));
        assert_eq!(plan.zero_copy_hint, LinuxZeroCopyHint::SharedTxRxPages);
    }

    #[test_case]
    fn queue_respects_depth_and_preserves_order() {
        let mut queue = LinuxBridgeQueue::new(LinuxBridgeChannel::Control, 1);
        let first = LinuxBridgeMessage::new(LinuxBridgeMessageKind::Discover, 1, LinuxBridgePayload::Empty);
        let second = LinuxBridgeMessage::new(LinuxBridgeMessageKind::Reset, 2, LinuxBridgePayload::Empty);

        assert!(queue.push(first).is_ok());
        assert!(queue.push(second).is_err());
        assert_eq!(queue.pop().map(|message| message.header.request_id), Some(1));
        assert!(queue.is_empty());
    }

    #[test_case]
    fn shim_context_builds_messages() {
        let plan = build_block_plan(DriverTransportKind::LibLinux, 0x3000, 0x100, 0x4000, 0x8000, 55);
        let ctx = LinuxShimDriverContext::new(plan);
        let discover = ctx.build_discovery_message(9);

        assert_eq!(discover.header.version, LinuxBridgeHeader::VERSION);
        assert_eq!(discover.header.kind, LinuxBridgeMessageKind::Discover);
    }

    #[test_case]
    fn zero_copy_request_maps_hint() {
        let request = make_data_request(
            11,
            LinuxIoRequestKind::NetTx,
            LinuxZeroCopyHint::PinnedScatterGather,
            vec![SharedBufferDescriptor::new(0, 128)],
        );

        assert_eq!(request.data_plane_hint, LinuxDataPlaneHint::PinnedPagesOnly);
        assert_eq!(request.payload.total_length(), 128);
    }
}