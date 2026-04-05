use super::super::linux::{LinuxResourcePlan, LinuxShimDeviceKind};
use super::super::{DriverCapabilitySet, DriverResources, DriverTransportKind, IrqGrant};
use super::telemetry::SideCarTelemetryStore;
use super::wire::{SideCarInterruptRoute, VirtioQueueSelector};

const LARGE_CONTROL_RING_DEPTH: usize = 256;
const LARGE_DATA_RING_DEPTH: usize = 1024;
const MEDIUM_CONTROL_RING_DEPTH: usize = 128;
const MEDIUM_DATA_RING_DEPTH: usize = 512;
const SMALL_CONTROL_RING_DEPTH: usize = 96;
const SMALL_DATA_RING_DEPTH: usize = 384;
const TINY_CONTROL_RING_DEPTH: usize = 64;
const TINY_DATA_RING_DEPTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideCarQueueClass {
    ControlPlane,
    DataPlane,
    Interrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarVmConfig {
    pub vm_id: u16,
    pub vcpu_count: u8,
    pub mem_bytes: usize,
    pub huge_pages: bool,
    pub isolate_iommu_domain: bool,
}

impl SideCarVmConfig {
    pub const fn new(vm_id: u16, vcpu_count: u8, mem_bytes: usize) -> Self {
        let vcpu_count = if vcpu_count == 0 { 1 } else { vcpu_count };
        Self {
            vm_id,
            vcpu_count,
            mem_bytes,
            huge_pages: true,
            isolate_iommu_domain: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideCarWorkloadProfile {
    pub mmio_bytes: usize,
    pub iova_bytes: usize,
    pub dma_pressure_hint: u8,
}

impl SideCarWorkloadProfile {
    pub const fn new(mmio_bytes: usize, iova_bytes: usize, dma_pressure_hint: u8) -> Self {
        Self {
            mmio_bytes,
            iova_bytes,
            dma_pressure_hint,
        }
    }

    pub const fn from_resource_lengths(mmio_bytes: usize, iova_bytes: usize) -> Self {
        let hint = if iova_bytes >= 0x8000 {
            100
        } else if iova_bytes >= 0x4000 {
            80
        } else if iova_bytes >= 0x2000 {
            60
        } else if iova_bytes >= 0x1000 {
            40
        } else {
            20
        };
        Self::new(mmio_bytes, iova_bytes, hint)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCarVmPlan {
    pub config: SideCarVmConfig,
    pub resources: DriverResources,
    pub queues: [VirtioQueueSelector; 4],
    pub irq_route: SideCarInterruptRoute,
    pub control_ring_depth: usize,
    pub data_ring_depth: usize,
}

impl SideCarVmPlan {
    pub fn for_linux_device(
        config: SideCarVmConfig,
        device_kind: LinuxShimDeviceKind,
        irq_vector: u32,
    ) -> Self {
        Self::for_linux_device_with_workload(
            config,
            device_kind,
            irq_vector,
            SideCarWorkloadProfile::new(0, 0, 0),
        )
    }

    pub fn for_linux_device_with_workload(
        config: SideCarVmConfig,
        device_kind: LinuxShimDeviceKind,
        irq_vector: u32,
        workload: SideCarWorkloadProfile,
    ) -> Self {
        let resources = DriverResources::new(DriverTransportKind::SideCarVm)
            .with_capabilities(
                DriverCapabilitySet::MMIO
                    | DriverCapabilitySet::DMA
                    | DriverCapabilitySet::IRQ
                    | DriverCapabilitySet::SHARED_MEMORY
                    | DriverCapabilitySet::CONTROL_QUEUE
                    | DriverCapabilitySet::RESET,
            )
            .add_irq(IrqGrant::new(irq_vector));

        let (base_control, base_data) = base_ring_depths_for(device_kind);
        let (control_ring_depth, data_ring_depth) = tuned_ring_depths(
            base_control,
            base_data,
            config.vcpu_count,
            config.mem_bytes,
            workload,
        );
        let irq_budget = tuned_irq_budget(device_kind, data_ring_depth, workload.dma_pressure_hint);

        Self {
            config,
            resources,
            queues: [
                VirtioQueueSelector::Control,
                VirtioQueueSelector::Completion,
                VirtioQueueSelector::Tx,
                VirtioQueueSelector::Rx,
            ],
            irq_route: SideCarInterruptRoute::new(irq_vector, irq_vector)
                .with_coalescing_budget(irq_budget),
            control_ring_depth,
            data_ring_depth,
        }
    }

    pub fn for_linux_device_with_telemetry(
        config: SideCarVmConfig,
        device_kind: LinuxShimDeviceKind,
        irq_vector: u32,
        base_workload: SideCarWorkloadProfile,
        telemetry: Option<&SideCarTelemetryStore>,
    ) -> Self {
        let workload = telemetry
            .map(|store| store.tuned_workload_profile(device_kind, base_workload))
            .unwrap_or(base_workload);

        Self::for_linux_device_with_workload(config, device_kind, irq_vector, workload)
    }

    pub fn from_linux_resource_plan(config: SideCarVmConfig, plan: &LinuxResourcePlan) -> Self {
        Self {
            config,
            resources: plan.resources.clone(),
            queues: [
                VirtioQueueSelector::Control,
                VirtioQueueSelector::Completion,
                VirtioQueueSelector::Tx,
                VirtioQueueSelector::Rx,
            ],
            irq_route: SideCarInterruptRoute::new(0, 0)
                .with_coalescing_budget(plan.irq_coalescing_budget),
            control_ring_depth: plan.control_queue_depth,
            data_ring_depth: plan.data_queue_depth,
        }
    }

    pub fn classify_queue(selector: VirtioQueueSelector) -> SideCarQueueClass {
        match selector {
            VirtioQueueSelector::Control | VirtioQueueSelector::Completion => {
                SideCarQueueClass::ControlPlane
            }
            VirtioQueueSelector::Tx | VirtioQueueSelector::Rx => SideCarQueueClass::DataPlane,
        }
    }
}

fn base_ring_depths_for(device_kind: LinuxShimDeviceKind) -> (usize, usize) {
    match device_kind {
        LinuxShimDeviceKind::Gpu
        | LinuxShimDeviceKind::WiFi
        | LinuxShimDeviceKind::Display
        | LinuxShimDeviceKind::Camera
        | LinuxShimDeviceKind::Audio => (LARGE_CONTROL_RING_DEPTH, LARGE_DATA_RING_DEPTH),
        LinuxShimDeviceKind::Network
        | LinuxShimDeviceKind::Ethernet
        | LinuxShimDeviceKind::Modem
        | LinuxShimDeviceKind::Usb
        | LinuxShimDeviceKind::Printer => (MEDIUM_CONTROL_RING_DEPTH, MEDIUM_DATA_RING_DEPTH),
        LinuxShimDeviceKind::Rtc
        | LinuxShimDeviceKind::SensorHub
        | LinuxShimDeviceKind::Sensor
        | LinuxShimDeviceKind::Input
        | LinuxShimDeviceKind::Dock
        | LinuxShimDeviceKind::Block
        | LinuxShimDeviceKind::Storage
        | LinuxShimDeviceKind::Nvme
        | LinuxShimDeviceKind::Firmware => (SMALL_CONTROL_RING_DEPTH, SMALL_DATA_RING_DEPTH),
        LinuxShimDeviceKind::Touch | LinuxShimDeviceKind::Gamepad => {
            (MEDIUM_CONTROL_RING_DEPTH, MEDIUM_DATA_RING_DEPTH)
        }
        LinuxShimDeviceKind::Bluetooth
        | LinuxShimDeviceKind::Nfc
        | LinuxShimDeviceKind::Tpm
        | LinuxShimDeviceKind::SmartCard
        | LinuxShimDeviceKind::Can
        | LinuxShimDeviceKind::Serial
        | LinuxShimDeviceKind::Generic => (TINY_CONTROL_RING_DEPTH, TINY_DATA_RING_DEPTH),
    }
}

fn tuned_ring_depths(
    base_control: usize,
    base_data: usize,
    vcpu_count: u8,
    mem_bytes: usize,
    workload: SideCarWorkloadProfile,
) -> (usize, usize) {
    let mut control = base_control;
    let mut data = base_data;

    if workload.mmio_bytes >= 0x200 {
        control = control.saturating_add(32);
    }
    if workload.iova_bytes >= 0x2000 {
        data = data.saturating_add(128);
    }
    if workload.iova_bytes >= 0x4000 {
        data = data.saturating_add(128);
    }

    if vcpu_count >= 4 {
        data = data.saturating_add(128);
    }

    if mem_bytes < 128 * 1024 * 1024 {
        control = control.saturating_sub(16).max(TINY_CONTROL_RING_DEPTH);
        data = data.saturating_sub(128).max(TINY_DATA_RING_DEPTH);
    }

    (control.clamp(64, 384), data.clamp(128, 2048))
}

fn tuned_irq_budget(device_kind: LinuxShimDeviceKind, data_ring_depth: usize, dma_hint: u8) -> usize {
    let mut budget: usize = if data_ring_depth >= 1024 {
        8
    } else if data_ring_depth >= 640 {
        6
    } else {
        4
    };

    if dma_hint >= 80 {
        budget = budget.saturating_add(2);
    } else if dma_hint <= 20 {
        budget = budget.saturating_sub(1).max(1);
    }

    if matches!(
        device_kind,
        LinuxShimDeviceKind::Input
            | LinuxShimDeviceKind::Touch
            | LinuxShimDeviceKind::Gamepad
            | LinuxShimDeviceKind::Rtc
    ) {
        budget = budget.saturating_sub(2).max(1);
    }

    budget.min(16)
}
