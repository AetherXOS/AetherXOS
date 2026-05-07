#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridRequestKind {
    Network,
    Block,
    Ethernet,
    Storage,
    Modem,
    Printer,
    Rtc,
    SensorHub,
    Gpu,
    WiFi,
    Camera,
    Audio,
    Sensor,
    Input,
    Touch,
    Gamepad,
    Bluetooth,
    Nfc,
    Tpm,
    Dock,
    Display,
    Usb,
    Can,
    Serial,
    Firmware,
    SmartCard,
    Nvme,
    WindowsPe,
    UserModeDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridRequestFamily {
    Network,
    Storage,
    Multimedia,
    Input,
    Security,
    Platform,
    Compatibility,
    Peripheral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridRequest {
    pub kind: HybridRequestKind,
    pub mmio_base: usize,
    pub mmio_length: usize,
    pub iova_base: usize,
    pub iova_length: usize,
    pub irq_vector: u32,
}

impl HybridRequest {
    pub const fn from_parts(
        kind: HybridRequestKind,
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self {
            kind,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        }
    }

    pub const fn zeroed(kind: HybridRequestKind) -> Self {
        Self::from_parts(kind, 0, 0, 0, 0, 0)
    }

    pub const fn device(
        kind: HybridRequestKind,
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::from_parts(kind, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn network(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Network, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn block(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Block, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn ethernet(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Ethernet,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn storage(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Storage,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn modem(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Modem,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn printer(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Printer,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn rtc(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Rtc, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn sensor_hub(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::SensorHub,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn windows_pe() -> Self {
        Self::zeroed(HybridRequestKind::WindowsPe)
    }

    pub const fn gpu(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Gpu, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn wifi(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::WiFi, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn user_mode_device(mmio_base: usize, mmio_length: usize, irq_vector: u32) -> Self {
        Self::from_parts(
            HybridRequestKind::UserModeDevice,
            mmio_base,
            mmio_length,
            0,
            0,
            irq_vector,
        )
    }

    pub const fn camera(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Camera, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn audio(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Audio, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn sensor(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Sensor, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn input(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Input, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn touch(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Touch, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn gamepad(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Gamepad, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn bluetooth(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Bluetooth, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn nfc(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Nfc, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn tpm(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Tpm, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn dock(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Dock, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn display(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Display, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn usb(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Usb, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn can(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Can, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn serial(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Serial, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn firmware(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Firmware,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn smart_card(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::SmartCard,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn nvme(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Nvme, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }
}
