use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
/// IOMMU (Input-Output Memory Management Unit) Interface.
/// Provides device isolation and DMA remapping.
/// Mandatory for "Secure Mode" to prevent malicious devices from overwriting kernel memory.
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
#[path = "iommu_support.rs"]
mod iommu_support;
#[path = "iommu_backend_ops.rs"]
mod iommu_backend_ops;
#[path = "iommu_mapping_ops.rs"]
mod iommu_mapping_ops;
use iommu_support::{
    can_map_page, is_page_aligned, next_ring_index, read_mmio_u32, read_mmio_u64,
    valid_device_address, virt_to_phys_local, write_mmio_u32, write_mmio_u64,
};
use iommu_backend_ops::*;
use iommu_mapping_ops::*;
#[path = "iommu_api.rs"]
mod iommu_api;
pub use iommu_api::{
    AmdVi, ArmSmmu, IntelVtd, Iommu, IommuFlags, attach_device_to_domain,
    device_domain, domain_stats, ensure_domain, flush_pending, init_platform_iommu,
    iova_mapping, is_initialized, is_iova_mapped, map_dma_page, map_dma_page_for_domain, stats,
    unmap_dma_page,
};

static IOMMU_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
const VTD_REG_GCMD: u64 = 0x18;
#[allow(dead_code)]
const VTD_REG_GSTS: u64 = 0x1C;
#[allow(dead_code)]
const VTD_REG_RTADDR: u64 = 0x20;
const VTD_REG_CCMD: u64 = 0x28;
const VTD_REG_ECAP: u64 = 0x10;

#[allow(dead_code)]
const VTD_GCMD_SRTP: u32 = 1 << 30;
#[allow(dead_code)]
const VTD_GSTS_RTPS: u32 = 1 << 30;
const VTD_CCMD_ICC: u64 = 1 << 63;
const VTD_CCMD_CIRG_GLOBAL: u64 = 0b01 << 61;
const VTD_IOTLB_IVT: u64 = 1 << 63;
const VTD_IOTLB_IIRG_GLOBAL: u64 = 0b01 << 60;

const VTD_CTX_PRESENT: u64 = 1 << 0;
const VTD_CTX_TRANSLATION_TYPE_0: u64 = 0 << 2;
const VTD_SLPT_READ: u64 = 1 << 0;
const VTD_SLPT_WRITE: u64 = 1 << 1;
const VTD_SLPT_EXEC: u64 = 1 << 2;

const AMDVI_CMD_BUFFER_BASE: u64 = 0x2000;
const AMDVI_CMD_BUFFER_HEAD: u64 = 0x2008;
const AMDVI_CMD_BUFFER_TAIL: u64 = 0x2010;
const AMDVI_INV_CMD_OPCODE_GLOBAL: u64 = 0x2;
const AMDVI_INV_CMD_OPCODE_DOMAIN: u64 = 0x3;
const AMDVI_INV_CMD_OPCODE_DEVICE: u64 = 0x4;
const IOMMU_MMIO_WAIT_TIMEOUT_SPINS: usize = 50_000;

#[repr(C, align(4096))]
struct VtdPage([u64; 512]);

impl VtdPage {
    fn new_zeroed() -> Self {
        Self([0; 512])
    }
}

#[repr(C, align(4096))]
struct AmdViCmdRing([u64; 512]);

impl AmdViCmdRing {
    fn new_zeroed() -> Self {
        Self([0; 512])
    }
}

#[derive(Clone, Copy)]
struct Mapping {
    domain_id: u16,
    phys: usize,
    flags: IommuFlags,
}

struct IommuState {
    backend: &'static str,
    hardware_mode: bool,
    dmar_drhd_register_bases: Vec<u64>,
    ivrs_ivhd_register_bases: Vec<u64>,
    domain_map: BTreeMap<u16, DomainState>,
    device_domain_map: BTreeMap<u16, u16>,
    vtd_root_table: Option<Box<VtdPage>>,
    vtd_context_tables: BTreeMap<u8, Box<VtdPage>>,
    amdvi_cmd_ring: Option<Box<AmdViCmdRing>>,
    amdvi_cmd_tail: u32,
    vtd_programmed_units: usize,
    vtd_hw_ready: bool,
    vtd_iotlb_inv_count: u64,
    amdvi_inv_count: u64,
    amdvi_inv_global_count: u64,
    amdvi_inv_domain_count: u64,
    amdvi_inv_device_count: u64,
    amdvi_inv_fallback_count: u64,
    amdvi_inv_timeout_count: u64,
    mappings: BTreeMap<usize, Mapping>,
    flush_count: u64,
    map_count: u64,
    unmap_count: u64,
}

struct DomainState {
    mappings: usize,
    attached_devices: usize,
    slpt_root: Option<Box<VtdPage>>,
    slpt_phys: u64,
    slpt_leaf_tables: BTreeMap<usize, Box<VtdPage>>,
    slpt_entries: usize,
}

impl Default for DomainState {
    fn default() -> Self {
        Self {
            mappings: 0,
            attached_devices: 0,
            slpt_root: None,
            slpt_phys: 0,
            slpt_leaf_tables: BTreeMap::new(),
            slpt_entries: 0,
        }
    }
}

impl IommuState {
    fn new() -> Self {
        Self {
            backend: "none",
            hardware_mode: false,
            dmar_drhd_register_bases: Vec::new(),
            ivrs_ivhd_register_bases: Vec::new(),
            domain_map: BTreeMap::new(),
            device_domain_map: BTreeMap::new(),
            vtd_root_table: None,
            vtd_context_tables: BTreeMap::new(),
            amdvi_cmd_ring: None,
            amdvi_cmd_tail: 0,
            vtd_programmed_units: 0,
            vtd_hw_ready: false,
            vtd_iotlb_inv_count: 0,
            amdvi_inv_count: 0,
            amdvi_inv_global_count: 0,
            amdvi_inv_domain_count: 0,
            amdvi_inv_device_count: 0,
            amdvi_inv_fallback_count: 0,
            amdvi_inv_timeout_count: 0,
            mappings: BTreeMap::new(),
            flush_count: 0,
            map_count: 0,
            unmap_count: 0,
        }
    }
}

lazy_static! {
    static ref IOMMU_STATE: Mutex<IommuState> = Mutex::new(IommuState::new());
}

#[derive(Debug, Clone, Copy)]
pub struct IommuStats {
    pub initialized: bool,
    pub backend: &'static str,
    pub hardware_mode: bool,
    pub vtd_units: usize,
    pub vtd_programmed_units: usize,
    pub vtd_hw_ready: bool,
    pub vtd_iotlb_inv_count: u64,
    pub amdvi_units: usize,
    pub amdvi_inv_count: u64,
    pub amdvi_inv_global_count: u64,
    pub amdvi_inv_domain_count: u64,
    pub amdvi_inv_device_count: u64,
    pub amdvi_inv_fallback_count: u64,
    pub amdvi_inv_timeout_count: u64,
    pub domains: usize,
    pub attached_devices: usize,
    pub mapping_count: usize,
    pub flush_count: u64,
    pub map_count: u64,
    pub unmap_count: u64,
}

pub const fn wait_timeout_spins() -> usize {
    IOMMU_MMIO_WAIT_TIMEOUT_SPINS
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl DeviceAddress {
    pub fn bdf(self) -> u16 {
        ((self.bus as u16) << 8) | ((self.device as u16) << 3) | (self.function as u16)
    }
}

