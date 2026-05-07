use super::types::*;
use super::telemetry::classify_syscall_semantics;

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
