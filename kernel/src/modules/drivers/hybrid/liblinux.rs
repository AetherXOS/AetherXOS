use alloc::vec::Vec;

use super::{
    LinuxIoRequest, LinuxIoRequestKind, SharedBufferDescriptor,
};

pub mod bridge;

pub use bridge::{
    classify_response, summarize_bridge_records, LibLinuxBridge, LinuxBridgeDispatchOutcome,
    LinuxBridgeDispatchRecord,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibLinuxBackendKind {
    StaticArchive,
    SharedObject,
    InKernelObjects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroCopyIoPolicy {
    Disabled,
    Preferred,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxSyscall {
    OpenAt,
    Read,
    Write,
    Ioctl,
    Mmap,
    Munmap,
    Socket,
    SendMsg,
    RecvMsg,
    Poll,
    EpollWait,
    Fsync,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxSyscallRequest {
    pub id: u64,
    pub syscall: LinuxSyscall,
    pub fd: i32,
    pub arg0: usize,
    pub arg1: usize,
    pub arg2: usize,
    pub policy: ZeroCopyIoPolicy,
    pub payload: Vec<SharedBufferDescriptor>,
}

impl LinuxSyscallRequest {
    pub fn new(id: u64, syscall: LinuxSyscall) -> Self {
        Self {
            id,
            syscall,
            fd: -1,
            arg0: 0,
            arg1: 0,
            arg2: 0,
            policy: ZeroCopyIoPolicy::Preferred,
            payload: Vec::new(),
        }
    }

    pub fn with_fd(mut self, fd: i32) -> Self {
        self.fd = fd;
        self
    }

    pub fn with_args(mut self, arg0: usize, arg1: usize, arg2: usize) -> Self {
        self.arg0 = arg0;
        self.arg1 = arg1;
        self.arg2 = arg2;
        self
    }

    pub fn with_policy(mut self, policy: ZeroCopyIoPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_payload(mut self, payload: Vec<SharedBufferDescriptor>) -> Self {
        self.payload = payload;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxSyscallResponse {
    pub id: u64,
    pub result: isize,
    pub errno: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxSyscallQueue {
    pub requests: Vec<LinuxSyscallRequest>,
    pub max_depth: usize,
}

impl LinuxSyscallQueue {
    pub fn new(max_depth: usize) -> Self {
        Self {
            requests: Vec::new(),
            max_depth: max_depth.max(1),
        }
    }

    pub fn push(&mut self, request: LinuxSyscallRequest) -> Result<(), LinuxSyscallRequest> {
        if self.requests.len() >= self.max_depth {
            return Err(request);
        }
        self.requests.push(request);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<LinuxSyscallRequest> {
        if self.requests.is_empty() {
            return None;
        }
        Some(self.requests.remove(0))
    }

    pub fn push_batch(
        &mut self,
        requests: Vec<LinuxSyscallRequest>,
    ) -> (usize, Vec<LinuxSyscallRequest>) {
        let mut accepted = 0usize;
        let mut rejected = Vec::new();
        for request in requests {
            match self.push(request) {
                Ok(()) => accepted += 1,
                Err(request) => rejected.push(request),
            }
        }
        (accepted, rejected)
    }

    pub fn drain_batch(&mut self, max_batch: usize) -> Vec<LinuxSyscallRequest> {
        let take = core::cmp::min(self.requests.len(), max_batch.max(1));
        self.requests.drain(0..take).collect()
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
}

pub trait LinuxSyscallDispatcher {
    fn dispatch_one<M: LinuxSyscallMapper>(
        &self,
        mapper: &M,
        request: LinuxSyscallRequest,
    ) -> (LinuxIoRequest, LinuxSyscallResponse);

    fn dispatch_batch<M: LinuxSyscallMapper>(
        &self,
        mapper: &M,
        queue: &mut LinuxSyscallQueue,
        max_batch: usize,
    ) -> Vec<(LinuxIoRequest, LinuxSyscallResponse)>;
}

pub trait LinuxSyscallMapper {
    fn to_io_request(&self, request: &LinuxSyscallRequest) -> LinuxIoRequest;
}

pub fn map_syscall_to_io_kind(syscall: LinuxSyscall) -> LinuxIoRequestKind {
    match syscall {
        LinuxSyscall::Read | LinuxSyscall::RecvMsg | LinuxSyscall::Poll | LinuxSyscall::EpollWait => {
            LinuxIoRequestKind::NetRx
        }
        LinuxSyscall::Write | LinuxSyscall::SendMsg | LinuxSyscall::Fsync => LinuxIoRequestKind::NetTx,
        LinuxSyscall::Ioctl => LinuxIoRequestKind::Control,
        LinuxSyscall::Mmap => LinuxIoRequestKind::BlockRead,
        LinuxSyscall::Munmap => LinuxIoRequestKind::BlockWrite,
        LinuxSyscall::OpenAt | LinuxSyscall::Socket => LinuxIoRequestKind::Control,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibLinuxSemanticClass {
    DataPath,
    ControlPath,
    MemoryMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibLinuxConformanceRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibLinuxConformanceReport {
    pub total_requests: usize,
    pub zero_copy_required: usize,
    pub zero_copy_eligible: usize,
    pub memory_mapping_ops: usize,
    pub control_ops: usize,
    pub data_ops: usize,
    pub high_risk_ops: usize,
    pub supported_ratio_pct: u8,
    pub risk: LibLinuxConformanceRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibLinuxDispatchSample {
    pub queue_depth: usize,
    pub batch_size: usize,
    pub success: usize,
    pub partial: usize,
    pub failure: usize,
}

impl LibLinuxDispatchSample {
    pub const fn new(
        queue_depth: usize,
        batch_size: usize,
        success: usize,
        partial: usize,
        failure: usize,
    ) -> Self {
        Self {
            queue_depth,
            batch_size,
            success,
            partial,
            failure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibLinuxTelemetrySummary {
    pub sample_count: usize,
    pub avg_queue_depth: usize,
    pub avg_batch_size: usize,
    pub total_success: usize,
    pub total_partial: usize,
    pub total_failure: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibLinuxTelemetryStore {
    max_samples: usize,
    samples: Vec<LibLinuxDispatchSample>,
}

impl LibLinuxTelemetryStore {
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples: max_samples.max(8),
            samples: Vec::new(),
        }
    }

    pub fn record_dispatch_sample(&mut self, sample: LibLinuxDispatchSample) {
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(sample);
    }

    pub fn summary(&self) -> Option<LibLinuxTelemetrySummary> {
        if self.samples.is_empty() {
            return None;
        }

        let mut queue_sum = 0usize;
        let mut batch_sum = 0usize;
        let mut total_success = 0usize;
        let mut total_partial = 0usize;
        let mut total_failure = 0usize;

        for sample in &self.samples {
            queue_sum += sample.queue_depth;
            batch_sum += sample.batch_size;
            total_success += sample.success;
            total_partial += sample.partial;
            total_failure += sample.failure;
        }

        Some(LibLinuxTelemetrySummary {
            sample_count: self.samples.len(),
            avg_queue_depth: queue_sum / self.samples.len(),
            avg_batch_size: batch_sum / self.samples.len(),
            total_success,
            total_partial,
            total_failure,
        })
    }

    pub fn recommended_batch_size(&self, queue_depth: usize, requested_max: usize) -> usize {
        let requested_max = requested_max.max(1);
        let queue_target = queue_depth.max(1).min(requested_max);

        let Some(summary) = self.summary() else {
            return queue_target;
        };

        let observed_total = summary
            .total_success
            .saturating_add(summary.total_partial)
            .saturating_add(summary.total_failure)
            .max(1);
        let failure_ratio_pct = (summary.total_failure * 100) / observed_total;
        let partial_ratio_pct = (summary.total_partial * 100) / observed_total;

        let mut suggested = queue_target.min(summary.avg_batch_size.max(1));

        if failure_ratio_pct >= 25 {
            suggested = suggested.saturating_sub(2).max(1);
        } else if partial_ratio_pct >= 35 {
            suggested = suggested.saturating_sub(1).max(1);
        } else if summary.avg_queue_depth >= summary.avg_batch_size.saturating_mul(2) {
            suggested = suggested.saturating_add(1).min(requested_max);
        }

        suggested.clamp(1, requested_max)
    }
}

pub fn classify_syscall_semantics(syscall: LinuxSyscall) -> LibLinuxSemanticClass {
    match syscall {
        LinuxSyscall::Mmap | LinuxSyscall::Munmap => LibLinuxSemanticClass::MemoryMap,
        LinuxSyscall::Ioctl | LinuxSyscall::OpenAt | LinuxSyscall::Socket => {
            LibLinuxSemanticClass::ControlPath
        }
        LinuxSyscall::Read
        | LinuxSyscall::Write
        | LinuxSyscall::SendMsg
        | LinuxSyscall::RecvMsg
        | LinuxSyscall::Poll
        | LinuxSyscall::EpollWait
        | LinuxSyscall::Fsync => LibLinuxSemanticClass::DataPath,
    }
}

pub fn is_zero_copy_eligible(request: &LinuxSyscallRequest) -> bool {
    !request.payload.is_empty()
        && matches!(
            request.syscall,
            LinuxSyscall::Read
                | LinuxSyscall::Write
                | LinuxSyscall::SendMsg
                | LinuxSyscall::RecvMsg
                | LinuxSyscall::Mmap
                | LinuxSyscall::Munmap
        )
}

pub fn conformance_report_for_requests(requests: &[LinuxSyscallRequest]) -> LibLinuxConformanceReport {
    if requests.is_empty() {
        return LibLinuxConformanceReport {
            total_requests: 0,
            zero_copy_required: 0,
            zero_copy_eligible: 0,
            memory_mapping_ops: 0,
            control_ops: 0,
            data_ops: 0,
            high_risk_ops: 0,
            supported_ratio_pct: 100,
            risk: LibLinuxConformanceRisk::Low,
        };
    }

    let mut zero_copy_required = 0usize;
    let mut zero_copy_eligible = 0usize;
    let mut memory_mapping_ops = 0usize;
    let mut control_ops = 0usize;
    let mut data_ops = 0usize;
    let mut high_risk_ops = 0usize;
    let mut supported_like_ops = 0usize;

    for request in requests {
        let class = classify_syscall_semantics(request.syscall);
        match class {
            LibLinuxSemanticClass::MemoryMap => memory_mapping_ops += 1,
            LibLinuxSemanticClass::ControlPath => control_ops += 1,
            LibLinuxSemanticClass::DataPath => data_ops += 1,
        }

        if request.policy == ZeroCopyIoPolicy::Required {
            zero_copy_required += 1;
            if is_zero_copy_eligible(request) {
                zero_copy_eligible += 1;
            } else {
                high_risk_ops += 1;
            }
        }

        let supported = match request.syscall {
            LinuxSyscall::OpenAt
            | LinuxSyscall::Read
            | LinuxSyscall::Write
            | LinuxSyscall::Ioctl
            | LinuxSyscall::Mmap
            | LinuxSyscall::Munmap
            | LinuxSyscall::Socket
            | LinuxSyscall::SendMsg
            | LinuxSyscall::RecvMsg
            | LinuxSyscall::Poll
            | LinuxSyscall::EpollWait
            | LinuxSyscall::Fsync => true,
        };

        if supported {
            supported_like_ops += 1;
        }

        if matches!(request.syscall, LinuxSyscall::Ioctl | LinuxSyscall::Mmap | LinuxSyscall::Munmap)
            && request.policy == ZeroCopyIoPolicy::Required
            && request.payload.is_empty()
        {
            high_risk_ops += 1;
        }
    }

    let supported_ratio_pct = ((supported_like_ops * 100) / requests.len()) as u8;
    let risk = if high_risk_ops >= 2 {
        LibLinuxConformanceRisk::High
    } else if high_risk_ops == 1 || memory_mapping_ops >= 2 {
        LibLinuxConformanceRisk::Medium
    } else {
        LibLinuxConformanceRisk::Low
    };

    LibLinuxConformanceReport {
        total_requests: requests.len(),
        zero_copy_required,
        zero_copy_eligible,
        memory_mapping_ops,
        control_ops,
        data_ops,
        high_risk_ops,
        supported_ratio_pct,
        risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::drivers::DriverTransportKind;
    use crate::modules::drivers::LinuxDataPlaneHint;

    #[test_case]
    fn mapper_applies_zero_copy_policy() {
        let request = LinuxSyscallRequest::new(10, LinuxSyscall::Write)
            .with_policy(ZeroCopyIoPolicy::Required)
            .with_payload(vec![SharedBufferDescriptor::new(0, 64)]);
        let mapper = LibLinuxBridge;
        let io_request = mapper.to_io_request(&request);

        assert_eq!(io_request.request_id, 10);
        assert_eq!(io_request.kind, LinuxIoRequestKind::NetTx);
        assert_eq!(io_request.data_plane_hint, LinuxDataPlaneHint::PinnedPagesOnly);
    }

    #[test_case]
    fn network_plan_uses_liblinux_transport() {
        let plan = LibLinuxBridge::plan_network(0x1000, 0x100, 0x2000, 0x2000, 50);
        assert_eq!(plan.transport, DriverTransportKind::LibLinux);
    }

    #[test_case]
    fn syscall_queue_and_dispatch_batch_work() {
        let bridge = LibLinuxBridge;
        let mapper = LibLinuxBridge;
        let mut queue = LinuxSyscallQueue::new(8);
        queue
            .push(
                LinuxSyscallRequest::new(1, LinuxSyscall::Write)
                    .with_payload(vec![SharedBufferDescriptor::new(0, 32)]),
            )
            .expect("queue should accept first request");
        queue
            .push(LinuxSyscallRequest::new(2, LinuxSyscall::Poll))
            .expect("queue should accept second request");

        let batch = bridge.dispatch_batch(&mapper, &mut queue, 4);
        assert_eq!(batch.len(), 2);
        assert!(queue.is_empty());
        assert_eq!(batch[0].1.id, 1);
        assert_eq!(batch[1].1.id, 2);
    }

    #[test_case]
    fn dispatch_batch_into_bridge_messages_produces_records() {
        let bridge = LibLinuxBridge;
        let mapper = LibLinuxBridge;
        let mut queue = LinuxSyscallQueue::new(4);
        queue
            .push(
                LinuxSyscallRequest::new(42, LinuxSyscall::Write)
                    .with_payload(vec![SharedBufferDescriptor::new(0, 128)]),
            )
            .expect("queue should accept request");

        let records = bridge.dispatch_batch_into_bridge(&mapper, &mut queue, 2);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].request_message.header.request_id, 42);
        assert_eq!(records[0].completion_message.header.request_id, 42);
    }

    #[test_case]
    fn classify_response_distinguishes_failure() {
        let ok = LibLinuxBridge::response_ok(1, 64);
        let fail = LibLinuxBridge::response_err(1, 5);
        assert!(matches!(LibLinuxBridge::classify_response(&ok), LinuxBridgeDispatchOutcome::Success));
        assert!(matches!(LibLinuxBridge::classify_response(&fail), LinuxBridgeDispatchOutcome::Failure));
    }

    #[test_case]
    fn push_batch_reports_backpressure() {
        let mut queue = LinuxSyscallQueue::new(2);
        let batch = vec![
            LinuxSyscallRequest::new(1, LinuxSyscall::Read),
            LinuxSyscallRequest::new(2, LinuxSyscall::Write),
            LinuxSyscallRequest::new(3, LinuxSyscall::Ioctl),
        ];

        let (accepted, rejected) = queue.push_batch(batch);
        assert_eq!(accepted, 2);
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].id, 3);
        assert_eq!(queue.len(), 2);
    }

    #[test_case]
    fn conformance_report_flags_zero_copy_risk_for_control_only_payloadless_ops() {
        let requests = vec![
            LinuxSyscallRequest::new(1, LinuxSyscall::Ioctl)
                .with_policy(ZeroCopyIoPolicy::Required),
            LinuxSyscallRequest::new(2, LinuxSyscall::Mmap)
                .with_policy(ZeroCopyIoPolicy::Required),
        ];

        let report = conformance_report_for_requests(&requests);
        assert_eq!(report.total_requests, 2);
        assert_eq!(report.zero_copy_required, 2);
        assert_eq!(report.zero_copy_eligible, 0);
        assert!(matches!(report.risk, LibLinuxConformanceRisk::High));
    }

    #[test_case]
    fn conformance_report_low_risk_for_data_path_zero_copy_payload() {
        let requests = vec![
            LinuxSyscallRequest::new(3, LinuxSyscall::Write)
                .with_policy(ZeroCopyIoPolicy::Required)
                .with_payload(vec![SharedBufferDescriptor::new(0, 128)]),
            LinuxSyscallRequest::new(4, LinuxSyscall::Read)
                .with_policy(ZeroCopyIoPolicy::Preferred)
                .with_payload(vec![SharedBufferDescriptor::new(1, 64)]),
        ];

        let report = conformance_report_for_requests(&requests);
        assert_eq!(report.data_ops, 2);
        assert_eq!(report.zero_copy_eligible, 1);
        assert!(matches!(report.risk, LibLinuxConformanceRisk::Low));
        assert_eq!(report.supported_ratio_pct, 100);
    }

    #[test_case]
    fn liblinux_telemetry_recommends_smaller_batch_when_failures_accumulate() {
        let mut telemetry = LibLinuxTelemetryStore::new(16);
        telemetry.record_dispatch_sample(LibLinuxDispatchSample::new(8, 8, 2, 1, 3));
        telemetry.record_dispatch_sample(LibLinuxDispatchSample::new(8, 8, 2, 1, 3));

        let recommended = telemetry.recommended_batch_size(8, 8);
        assert!(recommended < 8);
    }

    #[test_case]
    fn liblinux_telemetry_can_grow_batch_when_queue_outpaces_batch() {
        let mut telemetry = LibLinuxTelemetryStore::new(16);
        telemetry.record_dispatch_sample(LibLinuxDispatchSample::new(12, 4, 8, 0, 0));
        telemetry.record_dispatch_sample(LibLinuxDispatchSample::new(10, 4, 8, 0, 0));

        let recommended = telemetry.recommended_batch_size(12, 12);
        assert!(recommended >= 5);
    }
}
