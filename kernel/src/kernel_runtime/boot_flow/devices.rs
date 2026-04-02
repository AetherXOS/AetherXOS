use crate::kernel_runtime::KernelRuntime;

impl KernelRuntime {
    pub(super) fn init_pci_and_driver_runtime(&self) {
        use aethercore::kernel::startup::{StartupStage, mark_stage};

        let devices = self.enumerate_pci();
        mark_stage(StartupStage::PciEnumerated);
        self.attach_pci_to_iommu_domain(&devices);
        mark_stage(StartupStage::IommuAttached);
        self.init_drivers(&devices);
        mark_stage(StartupStage::DriversInit);
    }
}
