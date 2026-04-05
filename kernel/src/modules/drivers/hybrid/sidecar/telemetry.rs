use alloc::vec::Vec;

use super::super::linux::LinuxShimDeviceKind;
use super::vm::SideCarWorkloadProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarTelemetrySample {
    pub queue_fill_pct: u8,
    pub retry_count: u8,
    pub p99_latency_us: u32,
    pub irq_burst: u16,
}

impl SideCarTelemetrySample {
    pub const fn new(queue_fill_pct: u8, retry_count: u8, p99_latency_us: u32, irq_burst: u16) -> Self {
        Self {
            queue_fill_pct,
            retry_count,
            p99_latency_us,
            irq_burst,
        }
    }

    pub const fn clamped(self) -> Self {
        Self {
            queue_fill_pct: if self.queue_fill_pct > 100 { 100 } else { self.queue_fill_pct },
            retry_count: self.retry_count,
            p99_latency_us: self.p99_latency_us,
            irq_burst: self.irq_burst,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarTelemetrySummary {
    pub sample_count: usize,
    pub avg_queue_fill_pct: u8,
    pub avg_retry_count: u8,
    pub max_p99_latency_us: u32,
    pub max_irq_burst: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideCarSaturationLevel {
    Low,
    Nominal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SideCarTelemetryBucket {
    device_kind: LinuxShimDeviceKind,
    samples: Vec<SideCarTelemetrySample>,
    last_update_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCarTelemetrySnapshotBucket {
    pub device_kind: LinuxShimDeviceKind,
    pub samples: Vec<SideCarTelemetrySample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCarTelemetrySnapshot {
    pub buckets: Vec<SideCarTelemetrySnapshotBucket>,
}

impl SideCarTelemetrySnapshot {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.buckets.len() as u16).to_le_bytes());
        for bucket in &self.buckets {
            out.push(device_kind_to_u8(bucket.device_kind));
            out.extend_from_slice(&(bucket.samples.len() as u16).to_le_bytes());
            for sample in &bucket.samples {
                out.push(sample.queue_fill_pct);
                out.push(sample.retry_count);
                out.extend_from_slice(&sample.p99_latency_us.to_le_bytes());
                out.extend_from_slice(&sample.irq_burst.to_le_bytes());
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 2 {
            return None;
        }

        let mut cursor = 0usize;
        let bucket_count = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
        cursor += 2;

        let mut buckets = Vec::new();
        for _ in 0..bucket_count {
            if cursor >= bytes.len() {
                return None;
            }
            let device_kind = device_kind_from_u8(bytes[cursor])?;
            cursor += 1;

            if cursor + 2 > bytes.len() {
                return None;
            }
            let sample_count = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
            cursor += 2;

            let mut samples = Vec::new();
            for _ in 0..sample_count {
                if cursor + 8 > bytes.len() {
                    return None;
                }
                let queue_fill_pct = bytes[cursor];
                let retry_count = bytes[cursor + 1];
                let p99_latency_us = u32::from_le_bytes([
                    bytes[cursor + 2],
                    bytes[cursor + 3],
                    bytes[cursor + 4],
                    bytes[cursor + 5],
                ]);
                let irq_burst = u16::from_le_bytes([bytes[cursor + 6], bytes[cursor + 7]]);
                cursor += 8;

                samples.push(SideCarTelemetrySample::new(
                    queue_fill_pct,
                    retry_count,
                    p99_latency_us,
                    irq_burst,
                ));
            }

            buckets.push(SideCarTelemetrySnapshotBucket {
                device_kind,
                samples,
            });
        }

        if cursor != bytes.len() {
            return None;
        }

        Some(Self { buckets })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCarTelemetryStore {
    max_samples_per_device: usize,
    max_devices: usize,
    update_seq: u64,
    buckets: Vec<SideCarTelemetryBucket>,
}

impl SideCarTelemetryStore {
    pub fn new(max_samples_per_device: usize) -> Self {
        Self::new_with_limits(64, max_samples_per_device)
    }

    pub fn new_with_limits(max_devices: usize, max_samples_per_device: usize) -> Self {
        Self {
            max_samples_per_device: max_samples_per_device.max(4),
            max_devices: max_devices.max(1),
            update_seq: 0,
            buckets: Vec::new(),
        }
    }

    pub fn from_snapshot(
        snapshot: SideCarTelemetrySnapshot,
        max_devices: usize,
        max_samples_per_device: usize,
    ) -> Self {
        let mut store = Self::new_with_limits(max_devices, max_samples_per_device);
        for bucket in snapshot.buckets {
            for sample in bucket.samples {
                store.record(bucket.device_kind, sample);
            }
        }
        store
    }

    pub fn snapshot(&self) -> SideCarTelemetrySnapshot {
        SideCarTelemetrySnapshot {
            buckets: self
                .buckets
                .iter()
                .map(|bucket| SideCarTelemetrySnapshotBucket {
                    device_kind: bucket.device_kind,
                    samples: bucket.samples.clone(),
                })
                .collect(),
        }
    }

    pub fn device_count(&self) -> usize {
        self.buckets.len()
    }

    pub fn record(&mut self, device_kind: LinuxShimDeviceKind, sample: SideCarTelemetrySample) {
        self.update_seq = self.update_seq.saturating_add(1);
        let sample = sample.clamped();

        let idx = if let Some(pos) = self
            .buckets
            .iter()
            .position(|bucket| bucket.device_kind == device_kind)
        {
            pos
        } else {
            self.evict_if_needed_for_new_bucket();
            self.buckets.push(SideCarTelemetryBucket {
                device_kind,
                samples: Vec::new(),
                last_update_seq: self.update_seq,
            });
            self.buckets.len().saturating_sub(1)
        };

        let bucket = &mut self.buckets[idx];
        bucket.last_update_seq = self.update_seq;
        if bucket.samples.len() >= self.max_samples_per_device {
            bucket.samples.remove(0);
        }
        bucket.samples.push(sample);
    }

    pub fn summary_for(&self, device_kind: LinuxShimDeviceKind) -> Option<SideCarTelemetrySummary> {
        let bucket = self
            .buckets
            .iter()
            .find(|bucket| bucket.device_kind == device_kind)?;
        summarize_samples(&bucket.samples)
    }

    pub fn saturation_level_for(&self, device_kind: LinuxShimDeviceKind) -> SideCarSaturationLevel {
        let Some(summary) = self.summary_for(device_kind) else {
            return SideCarSaturationLevel::Nominal;
        };

        if summary.avg_queue_fill_pct >= 92 || summary.max_p99_latency_us >= 3_500 {
            SideCarSaturationLevel::Critical
        } else if summary.avg_queue_fill_pct >= 80 || summary.max_p99_latency_us >= 2_200 {
            SideCarSaturationLevel::High
        } else if summary.avg_queue_fill_pct <= 25 && summary.avg_retry_count == 0 {
            SideCarSaturationLevel::Low
        } else {
            SideCarSaturationLevel::Nominal
        }
    }

    pub fn tuned_workload_profile(
        &self,
        device_kind: LinuxShimDeviceKind,
        base: SideCarWorkloadProfile,
    ) -> SideCarWorkloadProfile {
        if let Some(summary) = self.summary_for(device_kind) {
            apply_telemetry_feedback(base, summary)
        } else {
            base
        }
    }

    pub fn evict_stale_devices(&mut self, keep_devices: usize) {
        let keep_devices = keep_devices.max(1);
        while self.buckets.len() > keep_devices {
            if let Some(stale_idx) = self
                .buckets
                .iter()
                .enumerate()
                .min_by_key(|(_, bucket)| bucket.last_update_seq)
                .map(|(idx, _)| idx)
            {
                self.buckets.remove(stale_idx);
            } else {
                break;
            }
        }
    }

    fn evict_if_needed_for_new_bucket(&mut self) {
        if self.buckets.len() < self.max_devices {
            return;
        }

        if let Some(stale_idx) = self
            .buckets
            .iter()
            .enumerate()
            .min_by_key(|(_, bucket)| bucket.last_update_seq)
            .map(|(idx, _)| idx)
        {
            self.buckets.remove(stale_idx);
        }
    }
}

pub fn apply_telemetry_feedback(
    base: SideCarWorkloadProfile,
    summary: SideCarTelemetrySummary,
) -> SideCarWorkloadProfile {
    let mut dma_hint = base.dma_pressure_hint as i16;
    let mut iova_bytes = base.iova_bytes;
    let mut mmio_bytes = base.mmio_bytes;

    if summary.avg_queue_fill_pct >= 80 {
        dma_hint += 20;
        iova_bytes = iova_bytes.saturating_add(0x1000);
    } else if summary.avg_queue_fill_pct <= 30 {
        dma_hint -= 10;
    }

    if summary.max_p99_latency_us >= 2_000 {
        dma_hint += 10;
        mmio_bytes = mmio_bytes.saturating_add(0x80);
    }

    if summary.avg_retry_count >= 2 {
        dma_hint += 15;
        iova_bytes = iova_bytes.saturating_add(0x800);
    }

    if summary.max_irq_burst >= 64 {
        dma_hint += 10;
    }

    SideCarWorkloadProfile::new(mmio_bytes, iova_bytes, dma_hint.clamp(0, 100) as u8)
}

fn summarize_samples(samples: &[SideCarTelemetrySample]) -> Option<SideCarTelemetrySummary> {
    if samples.is_empty() {
        return None;
    }

    let mut queue_sum = 0usize;
    let mut retry_sum = 0usize;
    let mut max_p99_latency_us = 0u32;
    let mut max_irq_burst = 0u16;

    for sample in samples {
        queue_sum += sample.queue_fill_pct as usize;
        retry_sum += sample.retry_count as usize;
        max_p99_latency_us = max_p99_latency_us.max(sample.p99_latency_us);
        max_irq_burst = max_irq_burst.max(sample.irq_burst);
    }

    Some(SideCarTelemetrySummary {
        sample_count: samples.len(),
        avg_queue_fill_pct: (queue_sum / samples.len()) as u8,
        avg_retry_count: (retry_sum / samples.len()) as u8,
        max_p99_latency_us,
        max_irq_burst,
    })
}


const fn device_kind_to_u8(kind: LinuxShimDeviceKind) -> u8 {
    match kind {
        LinuxShimDeviceKind::Network => 0,
        LinuxShimDeviceKind::Block => 1,
        LinuxShimDeviceKind::Ethernet => 2,
        LinuxShimDeviceKind::Storage => 3,
        LinuxShimDeviceKind::Modem => 4,
        LinuxShimDeviceKind::Printer => 5,
        LinuxShimDeviceKind::Rtc => 6,
        LinuxShimDeviceKind::SensorHub => 7,
        LinuxShimDeviceKind::Gpu => 8,
        LinuxShimDeviceKind::WiFi => 9,
        LinuxShimDeviceKind::Bluetooth => 10,
        LinuxShimDeviceKind::Nfc => 11,
        LinuxShimDeviceKind::Tpm => 12,
        LinuxShimDeviceKind::Dock => 13,
        LinuxShimDeviceKind::Display => 14,
        LinuxShimDeviceKind::Usb => 15,
        LinuxShimDeviceKind::Can => 16,
        LinuxShimDeviceKind::Serial => 17,
        LinuxShimDeviceKind::Firmware => 18,
        LinuxShimDeviceKind::SmartCard => 19,
        LinuxShimDeviceKind::Nvme => 20,
        LinuxShimDeviceKind::Touch => 21,
        LinuxShimDeviceKind::Gamepad => 22,
        LinuxShimDeviceKind::Camera => 23,
        LinuxShimDeviceKind::Audio => 24,
        LinuxShimDeviceKind::Sensor => 25,
        LinuxShimDeviceKind::Input => 26,
        LinuxShimDeviceKind::Generic => 27,
    }
}

const fn device_kind_from_u8(raw: u8) -> Option<LinuxShimDeviceKind> {
    match raw {
        0 => Some(LinuxShimDeviceKind::Network),
        1 => Some(LinuxShimDeviceKind::Block),
        2 => Some(LinuxShimDeviceKind::Ethernet),
        3 => Some(LinuxShimDeviceKind::Storage),
        4 => Some(LinuxShimDeviceKind::Modem),
        5 => Some(LinuxShimDeviceKind::Printer),
        6 => Some(LinuxShimDeviceKind::Rtc),
        7 => Some(LinuxShimDeviceKind::SensorHub),
        8 => Some(LinuxShimDeviceKind::Gpu),
        9 => Some(LinuxShimDeviceKind::WiFi),
        10 => Some(LinuxShimDeviceKind::Bluetooth),
        11 => Some(LinuxShimDeviceKind::Nfc),
        12 => Some(LinuxShimDeviceKind::Tpm),
        13 => Some(LinuxShimDeviceKind::Dock),
        14 => Some(LinuxShimDeviceKind::Display),
        15 => Some(LinuxShimDeviceKind::Usb),
        16 => Some(LinuxShimDeviceKind::Can),
        17 => Some(LinuxShimDeviceKind::Serial),
        18 => Some(LinuxShimDeviceKind::Firmware),
        19 => Some(LinuxShimDeviceKind::SmartCard),
        20 => Some(LinuxShimDeviceKind::Nvme),
        21 => Some(LinuxShimDeviceKind::Touch),
        22 => Some(LinuxShimDeviceKind::Gamepad),
        23 => Some(LinuxShimDeviceKind::Camera),
        24 => Some(LinuxShimDeviceKind::Audio),
        25 => Some(LinuxShimDeviceKind::Sensor),
        26 => Some(LinuxShimDeviceKind::Input),
        27 => Some(LinuxShimDeviceKind::Generic),
        _ => None,
    }
}
