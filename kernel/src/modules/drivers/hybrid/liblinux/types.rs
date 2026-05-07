use alloc::vec::Vec;
use super::super::{LinuxIoRequest, SharedBufferDescriptor};

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
