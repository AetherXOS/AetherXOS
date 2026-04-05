use alloc::vec::Vec;

use super::super::DriverErrorKind;
use super::super::DriverCompletion;
use super::super::DriverTransportKind;
use super::super::LinuxBridgeMessage;
use super::super::LinuxBridgeMessageKind;
use super::super::LinuxBridgePayload;
use super::super::LinuxIoRequest;
use super::super::linux::{make_data_request, LinuxZeroCopyHint};
use super::super::linux::{build_network_plan, LinuxResourcePlan};
use super::LinuxSyscall;
use super::LibLinuxConformanceReport;
use super::LinuxSyscallDispatcher;
use super::LinuxSyscallMapper;
use super::LinuxSyscallQueue;
use super::LinuxSyscallRequest;
use super::LinuxSyscallResponse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxBridgeDispatchRecord {
    pub io_request: LinuxIoRequest,
    pub syscall_response: LinuxSyscallResponse,
    pub request_message: LinuxBridgeMessage,
    pub completion_message: LinuxBridgeMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxBridgeDispatchOutcome {
    Success,
    Partial,
    Failure,
}

pub fn classify_response(response: &LinuxSyscallResponse) -> LinuxBridgeDispatchOutcome {
    if response.errno == 0 && response.result >= 0 {
        LinuxBridgeDispatchOutcome::Success
    } else if response.errno == 0 {
        LinuxBridgeDispatchOutcome::Partial
    } else {
        LinuxBridgeDispatchOutcome::Failure
    }
}

pub fn summarize_bridge_records(records: &[LinuxBridgeDispatchRecord]) -> (usize, usize, usize) {
    let mut success = 0usize;
    let mut partial = 0usize;
    let mut failure = 0usize;
    for record in records {
        match classify_response(&record.syscall_response) {
            LinuxBridgeDispatchOutcome::Success => success += 1,
            LinuxBridgeDispatchOutcome::Partial => partial += 1,
            LinuxBridgeDispatchOutcome::Failure => failure += 1,
        }
    }
    (success, partial, failure)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LibLinuxBridge;

impl LibLinuxBridge {
    pub fn plan_network(
        mmio_base: usize,
        mmio_len: usize,
        iova_base: usize,
        iova_len: usize,
        irq_vector: u32,
    ) -> LinuxResourcePlan {
        build_network_plan(
            DriverTransportKind::LibLinux,
            mmio_base,
            mmio_len,
            iova_base,
            iova_len,
            irq_vector,
        )
    }

    pub fn response_ok(id: u64, result: isize) -> LinuxSyscallResponse {
        LinuxSyscallResponse {
            id,
            result,
            errno: 0,
        }
    }

    pub fn response_err(id: u64, errno: i32) -> LinuxSyscallResponse {
        LinuxSyscallResponse {
            id,
            result: -1,
            errno,
        }
    }

    pub fn classify_response(response: &LinuxSyscallResponse) -> LinuxBridgeDispatchOutcome {
        classify_response(response)
    }

    pub fn summarize_bridge_records(
        records: &[LinuxBridgeDispatchRecord],
    ) -> (usize, usize, usize) {
        summarize_bridge_records(records)
    }

    pub fn conformance_report_for_requests(
        requests: &[LinuxSyscallRequest],
    ) -> LibLinuxConformanceReport {
        super::conformance_report_for_requests(requests)
    }

    pub fn dispatch_batch_into_bridge<M: LinuxSyscallMapper>(
        &self,
        mapper: &M,
        queue: &mut LinuxSyscallQueue,
        max_batch: usize,
    ) -> Vec<LinuxBridgeDispatchRecord> {
        let batch = self.dispatch_batch(mapper, queue, max_batch);
        let mut out = Vec::with_capacity(batch.len());
        for (io_request, syscall_response) in batch {
            let req_id = io_request.request_id;
            let request_message = LinuxBridgeMessage::new(
                LinuxBridgeMessageKind::NotifyQueue,
                req_id,
                LinuxBridgePayload::Request(io_request.clone()),
            );

            let completion = if syscall_response.errno == 0 {
                DriverCompletion::ok(req_id, io_request.payload.total_length())
            } else {
                DriverCompletion::err(req_id, DriverErrorKind::Io)
            };
            let completion_message = LinuxBridgeMessage::new(
                LinuxBridgeMessageKind::QueryStatus,
                req_id,
                LinuxBridgePayload::Completion(completion),
            );

            out.push(LinuxBridgeDispatchRecord {
                io_request,
                syscall_response,
                request_message,
                completion_message,
            });
        }
        out
    }
}

impl LinuxSyscallMapper for LibLinuxBridge {
    fn to_io_request(&self, request: &LinuxSyscallRequest) -> LinuxIoRequest {
        let kind = super::map_syscall_to_io_kind(request.syscall);
        let hint = match request.policy {
            super::ZeroCopyIoPolicy::Disabled => LinuxZeroCopyHint::None,
            super::ZeroCopyIoPolicy::Preferred => LinuxZeroCopyHint::ReadOnlyGrant,
            super::ZeroCopyIoPolicy::Required => LinuxZeroCopyHint::PinnedScatterGather,
        };
        make_data_request(request.id, kind, hint, request.payload.clone())
    }
}

impl LinuxSyscallDispatcher for LibLinuxBridge {
    fn dispatch_one<M: LinuxSyscallMapper>(
        &self,
        mapper: &M,
        request: LinuxSyscallRequest,
    ) -> (LinuxIoRequest, LinuxSyscallResponse) {
        let id = request.id;
        let io_req = mapper.to_io_request(&request);
        let response = match request.syscall {
            LinuxSyscall::OpenAt | LinuxSyscall::Socket => Self::response_ok(id, 3),
            LinuxSyscall::Read
            | LinuxSyscall::Write
            | LinuxSyscall::SendMsg
            | LinuxSyscall::RecvMsg
            | LinuxSyscall::Fsync => Self::response_ok(id, io_req.payload.total_length() as isize),
            LinuxSyscall::Poll | LinuxSyscall::EpollWait => Self::response_ok(id, 0),
            LinuxSyscall::Ioctl => Self::response_ok(id, 0),
            LinuxSyscall::Mmap => Self::response_ok(id, request.arg0 as isize),
            LinuxSyscall::Munmap => Self::response_ok(id, 0),
        };

        (io_req, response)
    }

    fn dispatch_batch<M: LinuxSyscallMapper>(
        &self,
        mapper: &M,
        queue: &mut LinuxSyscallQueue,
        max_batch: usize,
    ) -> Vec<(LinuxIoRequest, LinuxSyscallResponse)> {
        let drained = queue.drain_batch(max_batch);
        let mut out = Vec::with_capacity(drained.len());
        for request in drained {
            out.push(self.dispatch_one(mapper, request));
        }
        out
    }
}
