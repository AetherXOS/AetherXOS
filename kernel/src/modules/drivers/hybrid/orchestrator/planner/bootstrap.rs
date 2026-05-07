use alloc::vec::Vec;

use crate::modules::drivers::hybrid::liblinux::{
    LibLinuxBridge, LinuxBridgeDispatchRecord, LinuxSyscallMapper, LinuxSyscallQueue,
    LinuxSyscallRequest,
};
use crate::modules::drivers::hybrid::sidecar::{
    build_wire_notify, build_wire_probe, encode_payload, SideCarBootstrapState, SideCarPayload,
    SideCarTransport, SideCarVmConfig, VirtioQueueSelector,
};
use crate::modules::drivers::hybrid::{LinuxBridgeMessage, LinuxIoRequest};
use super::super::{BackendPreference, HybridExecutionPlan, HybridRequest, SideCarWireFrame};
use super::routing::plan;

pub fn build_sidecar_bootstrap_frames(
    request: &HybridRequest,
    sidecar_cfg: SideCarVmConfig,
    request_id_seed: u64,
) -> Option<Vec<SideCarWireFrame>> {
    let plan = plan(request, BackendPreference::SideCarFirst, sidecar_cfg)?;
    let sidecar = match plan {
        HybridExecutionPlan::SideCar(p) => p,
        _ => return None,
    };

    let mut out = Vec::new();
    out.push(SideCarWireFrame {
        header: build_wire_probe(sidecar.config.vm_id, request_id_seed),
        payload: SideCarPayload::Empty,
    });

    let notify_payload = SideCarPayload::QueueNotify {
        queue: VirtioQueueSelector::Control,
        desc_count: sidecar.control_ring_depth as u16,
        bytes: 0,
    };
    let notify_len = encode_payload(&notify_payload).len() as u32;
    out.push(SideCarWireFrame {
        header: build_wire_notify(
            sidecar.config.vm_id,
            request_id_seed.saturating_add(1),
            VirtioQueueSelector::Control,
            notify_len,
        ),
        payload: notify_payload,
    });
    Some(out)
}

pub fn submit_sidecar_frames<T: SideCarTransport>(
    transport: &mut T,
    frames: &[SideCarWireFrame],
) -> Result<(), T::Error> {
    for frame in frames {
        transport.send_wire(frame.header, frame.payload.clone())?;
    }
    Ok(())
}

pub fn drive_sidecar_bootstrap<T: SideCarTransport>(
    transport: &mut T,
    state: &mut SideCarBootstrapState,
    vm_id: u16,
    control_ring_depth: usize,
    current_tick: u32,
) -> Result<bool, T::Error> {
    if !state.should_retry(current_tick) {
        return Ok(false);
    }
    let Some((header, payload)) = state.current_frame(vm_id, control_ring_depth) else {
        return Ok(false);
    };
    transport.send_wire(header, payload)?;
    Ok(true)
}

pub fn advance_sidecar_bootstrap_from_bridge_message(
    state: &mut SideCarBootstrapState,
    message: &LinuxBridgeMessage,
    current_tick: u32,
) -> bool {
    state.apply_bridge_message(message, current_tick)
}

pub fn dispatch_liblinux_queue_to_bridge(
    queue: &mut LinuxSyscallQueue,
    max_batch: usize,
) -> Vec<LinuxBridgeDispatchRecord> {
    LibLinuxBridge.dispatch_batch_into_bridge(&LibLinuxBridge, queue, max_batch)
}

pub fn map_liblinux_syscall(request: &LinuxSyscallRequest) -> LinuxIoRequest {
    <LibLinuxBridge as LinuxSyscallMapper>::to_io_request(&LibLinuxBridge, request)
}
