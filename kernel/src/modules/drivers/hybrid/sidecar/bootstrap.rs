use super::super::LinuxBridgeMessage;
use super::super::LinuxBridgeMessageKind;
use super::super::LinuxBridgePayload;
use super::build_wire_notify;
use super::build_wire_probe;
use super::encode_payload;
use super::SideCarPayload;
use super::SideCarWireHeader;
use super::VirtioQueueSelector;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideCarBootstrapPhase {
    Probe,
    ControlNotify,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarBootstrapSummary {
    pub phase: SideCarBootstrapPhase,
    pub attempt: u8,
    pub next_retry_tick: u32,
    pub should_retry_now: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarRetryPolicy {
    pub max_attempts: u8,
    pub base_backoff_ticks: u32,
    pub max_backoff_ticks: u32,
}

impl SideCarRetryPolicy {
    pub const fn conservative() -> Self {
        Self {
            max_attempts: 3,
            base_backoff_ticks: 10,
            max_backoff_ticks: 200,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarBootstrapState {
    pub phase: SideCarBootstrapPhase,
    pub attempt: u8,
    pub next_retry_tick: u32,
    pub request_id_seed: u64,
    pub policy: SideCarRetryPolicy,
}

impl SideCarBootstrapState {
    pub const fn new(request_id_seed: u64, policy: SideCarRetryPolicy) -> Self {
        Self {
            phase: SideCarBootstrapPhase::Probe,
            attempt: 0,
            next_retry_tick: 0,
            request_id_seed,
            policy,
        }
    }

    pub fn current_frame(
        &self,
        vm_id: u16,
        control_ring_depth: usize,
    ) -> Option<(SideCarWireHeader, SideCarPayload)> {
        match self.phase {
            SideCarBootstrapPhase::Probe => Some((
                build_wire_probe(vm_id, self.request_id_seed),
                SideCarPayload::Empty,
            )),
            SideCarBootstrapPhase::ControlNotify => {
                let payload = SideCarPayload::QueueNotify {
                    queue: VirtioQueueSelector::Control,
                    desc_count: control_ring_depth as u16,
                    bytes: 0,
                };
                let payload_len = encode_payload(&payload).len() as u32;
                Some((
                    build_wire_notify(
                        vm_id,
                        self.request_id_seed.saturating_add(1),
                        VirtioQueueSelector::Control,
                        payload_len,
                    ),
                    payload,
                ))
            }
            SideCarBootstrapPhase::Completed => None,
        }
    }

    pub fn mark_success(&mut self) {
        self.attempt = 0;
        self.next_retry_tick = 0;
        self.phase = match self.phase {
            SideCarBootstrapPhase::Probe => SideCarBootstrapPhase::ControlNotify,
            SideCarBootstrapPhase::ControlNotify => SideCarBootstrapPhase::Completed,
            SideCarBootstrapPhase::Completed => SideCarBootstrapPhase::Completed,
        };
    }

    pub fn mark_failure(&mut self, current_tick: u32) -> bool {
        if self.attempt >= self.policy.max_attempts {
            return false;
        }

        self.attempt = self.attempt.saturating_add(1);
        let shift = (self.attempt.saturating_sub(1) as u32).min(16);
        let factor = 1u32.checked_shl(shift).unwrap_or(u32::MAX);
        let raw_backoff = self.policy.base_backoff_ticks.saturating_mul(factor);
        let backoff = raw_backoff.min(self.policy.max_backoff_ticks);
        self.next_retry_tick = current_tick.saturating_add(backoff);
        true
    }

    pub fn should_retry(&self, current_tick: u32) -> bool {
        self.phase != SideCarBootstrapPhase::Completed && current_tick >= self.next_retry_tick
    }

    pub fn apply_bridge_message(
        &mut self,
        message: &LinuxBridgeMessage,
        current_tick: u32,
    ) -> bool {
        match (&message.header.kind, &message.payload) {
            (LinuxBridgeMessageKind::QueryStatus, LinuxBridgePayload::Completion(completion)) => {
                match completion.status {
                    Ok(()) => self.mark_success(),
                    Err(_) => {
                        let _ = self.mark_failure(current_tick);
                    }
                }
                true
            }
            (LinuxBridgeMessageKind::QueryStatus, LinuxBridgePayload::Error(_)) => {
                self.mark_failure(current_tick)
            }
            (LinuxBridgeMessageKind::InterruptAck, LinuxBridgePayload::Empty) => {
                self.mark_success();
                true
            }
            _ => false,
        }
    }

    pub fn summary(&self, current_tick: u32) -> SideCarBootstrapSummary {
        SideCarBootstrapSummary {
            phase: self.phase,
            attempt: self.attempt,
            next_retry_tick: self.next_retry_tick,
            should_retry_now: self.should_retry(current_tick),
        }
    }
}
