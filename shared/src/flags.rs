use crate::define_flags;

define_flags! {
    pub struct KernelFeatures: u64 {
        NONE            = 0,
        VFS             = 1 << 0,
        DRIVERS         = 1 << 1,
        NET             = 1 << 2,
        SMP             = 1 << 3,
        TEST_MODE       = 1 << 4,
        GRAPHICS        = 1 << 5,
        USB             = 1 << 6,
        PCI             = 1 << 7,
        ACPI            = 1 << 8,
        LOGGING         = 1 << 9,
        SCHEDULER       = 1 << 10,
        MEMORY_MGMT     = 1 << 11,
        SYSCALLS        = 1 << 12,
        KVM             = 1 << 13,
        HARDENING       = 1 << 14,
        IO_URING        = 1 << 15,
    }
}

impl core::str::FromStr for KernelFeatures {
    type Err = alloc::string::String;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(Self::from_cargo_features(&s.split(',').collect::<alloc::vec::Vec<_>>()))
    }
}

impl KernelFeatures {
    pub fn to_cargo_features(self) -> alloc::vec::Vec<&'static str> {
        let mut features = alloc::vec::Vec::new();
        if self.contains(Self::VFS) { features.push("vfs"); }
        if self.contains(Self::DRIVERS) { features.push("drivers"); }
        if self.contains(Self::NET) { features.push("net"); }
        if self.contains(Self::SMP) { features.push("smp"); }
        if self.contains(Self::TEST_MODE) { features.push("kernel_test_mode"); }
        if self.contains(Self::GRAPHICS) { features.push("graphics"); }
        if self.contains(Self::USB) { features.push("usb"); }
        if self.contains(Self::PCI) { features.push("pci"); }
        if self.contains(Self::ACPI) { features.push("acpi"); }
        if self.contains(Self::LOGGING) { features.push("logging"); }
        if self.contains(Self::SCHEDULER) { features.push("scheduler"); }
        if self.contains(Self::MEMORY_MGMT) { features.push("memory_mgmt"); }
        if self.contains(Self::SYSCALLS) { features.push("syscalls"); }
        if self.contains(Self::KVM) { features.push("kvm"); }
        if self.contains(Self::HARDENING) { features.push("hardening"); }
        if self.contains(Self::IO_URING) { features.push("io_uring"); }
        features
    }

    pub fn from_cargo_features(features: &[&str]) -> Self {
        let mut flags = Self::NONE;
        for f in features {
            match *f {
                "vfs" => flags |= Self::VFS,
                "drivers" => flags |= Self::DRIVERS,
                "net" => flags |= Self::NET,
                "smp" => flags |= Self::SMP,
                "kernel_test_mode" => flags |= Self::TEST_MODE,
                "graphics" => flags |= Self::GRAPHICS,
                "usb" => flags |= Self::USB,
                "pci" => flags |= Self::PCI,
                "acpi" => flags |= Self::ACPI,
                "logging" => flags |= Self::LOGGING,
                "scheduler" => flags |= Self::SCHEDULER,
                "memory_mgmt" => flags |= Self::MEMORY_MGMT,
                "syscalls" => flags |= Self::SYSCALLS,
                "kvm" => flags |= Self::KVM,
                "hardening" => flags |= Self::HARDENING,
                "io_uring" => flags |= Self::IO_URING,
                _ => {}
            }
        }
        flags
    }
}
