use alloc::vec::Vec;
use super::types::*;
use super::super::HybridRequestKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibLinuxTelemetryStore {
    max_samples: usize,
    samples: Vec<LibLinuxDispatchSample>,
    data_path_samples: Vec<LibLinuxDispatchSample>,
    control_path_samples: Vec<LibLinuxDispatchSample>,
    memory_map_samples: Vec<LibLinuxDispatchSample>,
}

impl LibLinuxTelemetryStore {
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples: max_samples.max(8),
            samples: Vec::new(),
            data_path_samples: Vec::new(),
            control_path_samples: Vec::new(),
            memory_map_samples: Vec::new(),
        }
    }

    pub fn record_dispatch_sample(&mut self, sample: LibLinuxDispatchSample) {
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(sample);
    }

    pub fn record_dispatch_sample_for_syscall(
        &mut self,
        syscall: LinuxSyscall,
        sample: LibLinuxDispatchSample,
    ) {
        self.record_dispatch_sample(sample);
        let family_samples = match classify_syscall_semantics(syscall) {
            LibLinuxSemanticClass::DataPath => &mut self.data_path_samples,
            LibLinuxSemanticClass::ControlPath => &mut self.control_path_samples,
            LibLinuxSemanticClass::MemoryMap => &mut self.memory_map_samples,
        };
        if family_samples.len() >= self.max_samples {
            family_samples.remove(0);
        }
        family_samples.push(sample);
    }

    pub fn summary(&self) -> Option<LibLinuxTelemetrySummary> {
        summarize_dispatch_samples(&self.samples)
    }

    pub fn recommended_batch_size(&self, queue_depth: usize, requested_max: usize) -> usize {
        let requested_max = requested_max.max(1);
        let queue_target = queue_depth.max(1).min(requested_max);

        let Some(summary) = self.summary() else {
            return queue_target;
        };

        recommended_batch_size_from_summary(summary, queue_target, requested_max)
    }

    pub fn recommended_batch_size_for_syscall(
        &self,
        syscall: LinuxSyscall,
        queue_depth: usize,
        requested_max: usize,
    ) -> usize {
        let requested_max = requested_max.max(1);
        let queue_target = queue_depth.max(1).min(requested_max);

        let family_summary = match classify_syscall_semantics(syscall) {
            LibLinuxSemanticClass::DataPath => summarize_dispatch_samples(&self.data_path_samples),
            LibLinuxSemanticClass::ControlPath => {
                summarize_dispatch_samples(&self.control_path_samples)
            }
            LibLinuxSemanticClass::MemoryMap => summarize_dispatch_samples(&self.memory_map_samples),
        };

        if let Some(summary) = family_summary {
            return recommended_batch_size_from_summary(summary, queue_target, requested_max);
        }

        self.recommended_batch_size(queue_depth, requested_max)
    }

    pub fn recommended_batch_size_for_request_kind(
        &self,
        kind: HybridRequestKind,
        queue_depth: usize,
        requested_max: usize,
    ) -> usize {
        let requested_max = requested_max.max(1);
        let queue_target = queue_depth.max(1).min(requested_max);
        let family_summary = match semantic_class_for_request_kind(kind) {
            LibLinuxSemanticClass::DataPath => summarize_dispatch_samples(&self.data_path_samples),
            LibLinuxSemanticClass::ControlPath => {
                summarize_dispatch_samples(&self.control_path_samples)
            }
            LibLinuxSemanticClass::MemoryMap => summarize_dispatch_samples(&self.memory_map_samples),
        };

        family_summary
            .map(|summary| recommended_batch_size_from_summary(summary, queue_target, requested_max))
            .unwrap_or_else(|| self.recommended_batch_size(queue_depth, requested_max))
    }

    pub fn family_failure_pressure_for_request_kind(&self, kind: HybridRequestKind) -> u8 {
        let summary = match semantic_class_for_request_kind(kind) {
            LibLinuxSemanticClass::DataPath => summarize_dispatch_samples(&self.data_path_samples),
            LibLinuxSemanticClass::ControlPath => {
                summarize_dispatch_samples(&self.control_path_samples)
            }
            LibLinuxSemanticClass::MemoryMap => summarize_dispatch_samples(&self.memory_map_samples),
        };

        let Some(summary) = summary else {
            return 0;
        };

        let observed_total = summary
            .total_success
            .saturating_add(summary.total_partial)
            .saturating_add(summary.total_failure)
            .max(1);
        ((summary.total_failure * 100) / observed_total) as u8
    }

    pub fn family_pressure_is_high(&self, kind: HybridRequestKind, threshold_pct: u8) -> bool {
        self.family_failure_pressure_for_request_kind(kind) >= threshold_pct
    }
}

pub fn summarize_dispatch_samples(samples: &[LibLinuxDispatchSample]) -> Option<LibLinuxTelemetrySummary> {
    if samples.is_empty() {
        return None;
    }

    let mut queue_sum = 0usize;
    let mut batch_sum = 0usize;
    let mut total_success = 0usize;
    let mut total_partial = 0usize;
    let mut total_failure = 0usize;

    for sample in samples {
        queue_sum += sample.queue_depth;
        batch_sum += sample.batch_size;
        total_success += sample.success;
        total_partial += sample.partial;
        total_failure += sample.failure;
    }

    Some(LibLinuxTelemetrySummary {
        sample_count: samples.len(),
        avg_queue_depth: queue_sum / samples.len(),
        avg_batch_size: batch_sum / samples.len(),
        total_success,
        total_partial,
        total_failure,
    })
}

pub fn recommended_batch_size_from_summary(
    summary: LibLinuxTelemetrySummary,
    queue_target: usize,
    requested_max: usize,
) -> usize {
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

pub fn semantic_class_for_request_kind(kind: HybridRequestKind) -> LibLinuxSemanticClass {
    match kind {
        HybridRequestKind::Network
        | HybridRequestKind::Ethernet
        | HybridRequestKind::WiFi
        | HybridRequestKind::Bluetooth
        | HybridRequestKind::Nfc
        | HybridRequestKind::Modem
        | HybridRequestKind::Can
        | HybridRequestKind::Gpu
        | HybridRequestKind::Camera
        | HybridRequestKind::Audio
        | HybridRequestKind::Display
        | HybridRequestKind::Touch
        | HybridRequestKind::Gamepad
        | HybridRequestKind::Input
        | HybridRequestKind::Sensor
        | HybridRequestKind::SensorHub
        | HybridRequestKind::Rtc => LibLinuxSemanticClass::DataPath,
        HybridRequestKind::Block
        | HybridRequestKind::Storage
        | HybridRequestKind::Nvme
        | HybridRequestKind::Printer
        | HybridRequestKind::Usb
        | HybridRequestKind::Serial => LibLinuxSemanticClass::MemoryMap,
        HybridRequestKind::Tpm
        | HybridRequestKind::SmartCard
        | HybridRequestKind::UserModeDevice
        | HybridRequestKind::Dock
        | HybridRequestKind::Firmware
        | HybridRequestKind::WindowsPe => LibLinuxSemanticClass::ControlPath,
    }
}
