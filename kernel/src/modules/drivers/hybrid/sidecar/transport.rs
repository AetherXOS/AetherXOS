use alloc::collections::VecDeque;

use super::super::linux::LinuxShimDriverContext;
use super::super::{DriverResources, LinuxBridgeMessage, LinuxBridgeMessageKind, LinuxBridgePayload};
use super::wire::{
    encode_payload, SideCarInterruptRoute, SideCarOpcode, SideCarPayload, SideCarWireError,
    SideCarWireHeader, VirtioQueueSelector,
};

pub trait SideCarTransport {
    type Error;

    fn send_wire(
        &mut self,
        header: SideCarWireHeader,
        payload: SideCarPayload,
    ) -> Result<(), Self::Error>;
    fn send_control(&mut self, message: LinuxBridgeMessage) -> Result<(), Self::Error>;
    fn recv_completion(&mut self) -> Result<Option<LinuxBridgeMessage>, Self::Error>;
    fn notify_queue(&mut self, queue: VirtioQueueSelector) -> Result<(), Self::Error>;
    fn inject_irq(&mut self, route: SideCarInterruptRoute) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemorySideCarTransport {
    control_tx: VecDeque<(SideCarWireHeader, SideCarPayload)>,
    completion_rx: VecDeque<LinuxBridgeMessage>,
}

impl InMemorySideCarTransport {
    pub fn new() -> Self {
        Self {
            control_tx: VecDeque::new(),
            completion_rx: VecDeque::new(),
        }
    }

    pub fn push_completion(&mut self, message: LinuxBridgeMessage) {
        self.completion_rx.push_back(message);
    }

    pub fn pop_wire_frame(&mut self) -> Option<(SideCarWireHeader, SideCarPayload)> {
        self.control_tx.pop_front()
    }

    pub fn send_packed(
        &mut self,
        header: SideCarWireHeader,
        payload: SideCarPayload,
    ) -> Result<(), SideCarWireError> {
        let payload_bytes = encode_payload(&payload);
        if payload_bytes.len() as u32 != header.payload_len {
            return Err(SideCarWireError::InvalidPayload);
        }
        self.control_tx.push_back((header, payload));
        Ok(())
    }
}

impl Default for InMemorySideCarTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl SideCarTransport for InMemorySideCarTransport {
    type Error = SideCarWireError;

    fn send_wire(
        &mut self,
        header: SideCarWireHeader,
        payload: SideCarPayload,
    ) -> Result<(), Self::Error> {
        self.send_packed(header, payload)
    }

    fn send_control(&mut self, message: LinuxBridgeMessage) -> Result<(), Self::Error> {
        let payload_len = message.header.payload_len;
        let header = SideCarWireHeader::new(
            SideCarOpcode::NotifyQueue,
            VirtioQueueSelector::Control,
            message.header.request_id,
            0,
            payload_len,
        );
        self.send_packed(header, SideCarPayload::Empty)
    }

    fn recv_completion(&mut self) -> Result<Option<LinuxBridgeMessage>, Self::Error> {
        Ok(self.completion_rx.pop_front())
    }

    fn notify_queue(&mut self, queue: VirtioQueueSelector) -> Result<(), Self::Error> {
        let payload = SideCarPayload::QueueNotify {
            queue,
            desc_count: 0,
            bytes: 0,
        };
        let payload_len = encode_payload(&payload).len() as u32;
        let header = SideCarWireHeader::new(SideCarOpcode::NotifyQueue, queue, 0, 0, payload_len);
        self.send_packed(header, payload)
    }

    fn inject_irq(&mut self, route: SideCarInterruptRoute) -> Result<(), Self::Error> {
        let payload = SideCarPayload::IrqRoute(route);
        let payload_len = encode_payload(&payload).len() as u32;
        let header = SideCarWireHeader::new(
            SideCarOpcode::InjectIrq,
            VirtioQueueSelector::Completion,
            0,
            0,
            payload_len,
        );
        self.send_packed(header, payload)
    }
}

pub fn build_sidecar_attach_message(request_id: u64, resources: DriverResources) -> LinuxBridgeMessage {
    LinuxBridgeMessage::new(
        LinuxBridgeMessageKind::Attach,
        request_id,
        LinuxBridgePayload::Resources(resources),
    )
}

pub fn build_sidecar_irq_ack(request_id: u64) -> LinuxBridgeMessage {
    LinuxBridgeMessage::new(
        LinuxBridgeMessageKind::InterruptAck,
        request_id,
        LinuxBridgePayload::Empty,
    )
}

pub fn schedule_sidecar_bootstrap(
    context: &mut LinuxShimDriverContext,
    request_id_seed: u64,
) -> Result<(), LinuxBridgeMessage> {
    context.enqueue_control(context.build_discovery_message(request_id_seed))?;
    context.enqueue_control(context.build_reset(request_id_seed.saturating_add(1)))?;
    Ok(())
}
