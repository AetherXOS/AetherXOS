mod firmware;
mod virtualization;

pub(crate) use self::firmware::{init_acpi_discovery, init_iommu_discovery, log_dtb_discovery};
pub(crate) use self::virtualization::init_virtualization_bootstrap;
