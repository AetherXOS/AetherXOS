use super::*;

pub trait Iommu {
    /// Initialize the IOMMU hardware (VT-d or SMMU).
    fn init(&mut self);

    /// Map a physical page to a device-visible IO-Virtual Address (IOVA).
    /// Safe drivers only see their own isolated memory regions.
    /// Flags: R/W/X permissions.
    fn map_page(&mut self, phys: usize, iova: usize, flags: IommuFlags);

    /// Unmap a page.
    fn unmap_page(&mut self, iova: usize);

    /// Flush the IOTLB (I/O Translation Lookaside Buffer) to apply changes.
    fn flush_iotlb(&mut self);
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct IommuFlags: u64 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2; // Some IOMMUs support this
        const NON_SNOOP = 1 << 3; // Cache coherency
    }
}

pub struct IntelVtd;
impl Iommu for IntelVtd {
    fn init(&mut self) {
        let mut state = IOMMU_STATE.lock();
        state.backend = "intel-vtd";
    }
    fn map_page(&mut self, p: usize, v: usize, f: IommuFlags) {
        let _ = map_page_internal(p, v, f);
    }
    fn unmap_page(&mut self, v: usize) {
        let _ = unmap_page_internal(v);
    }
    fn flush_iotlb(&mut self) {
        flush_iotlb_internal();
    }
}

pub struct ArmSmmu;
impl Iommu for ArmSmmu {
    fn init(&mut self) {
        let mut state = IOMMU_STATE.lock();
        state.backend = "arm-smmu";
    }
    fn map_page(&mut self, p: usize, v: usize, f: IommuFlags) {
        let _ = map_page_internal(p, v, f);
    }
    fn unmap_page(&mut self, v: usize) {
        let _ = unmap_page_internal(v);
    }
    fn flush_iotlb(&mut self) {
        flush_iotlb_internal();
    }
}

pub struct AmdVi;
impl Iommu for AmdVi {
    fn init(&mut self) {
        let mut state = IOMMU_STATE.lock();
        state.backend = "amd-vi";
    }
    fn map_page(&mut self, p: usize, v: usize, f: IommuFlags) {
        let _ = map_page_internal(p, v, f);
    }
    fn unmap_page(&mut self, v: usize) {
        let _ = unmap_page_internal(v);
    }
    fn flush_iotlb(&mut self) {
        flush_iotlb_internal();
    }
}

pub fn init_platform_iommu() {
    #[cfg(target_arch = "x86_64")]
    {
        let info = crate::hal::x86_64::acpi::discover_iommu_info();
        let units = crate::hal::x86_64::acpi::discover_iommu_units();
        if info.dmar_drhd_units > 0 {
            let mut iommu = IntelVtd;
            iommu.init();
            let mut state = IOMMU_STATE.lock();
            state.backend = "intel-vtd-hw";
            state.hardware_mode = true;
            state.dmar_drhd_register_bases = units.dmar_drhd_register_bases;
            state.ivrs_ivhd_register_bases.clear();
            state.domain_map.clear();
            state.device_domain_map.clear();
            state.vtd_root_table = Some(Box::new(VtdPage::new_zeroed()));
            state.vtd_context_tables.clear();
            state.amdvi_cmd_ring = None;
            state.amdvi_cmd_tail = 0;
            state.vtd_iotlb_inv_count = 0;
            state.amdvi_inv_count = 0;
            state.amdvi_inv_global_count = 0;
            state.amdvi_inv_domain_count = 0;
            state.amdvi_inv_device_count = 0;
            state.amdvi_inv_fallback_count = 0;
            state.amdvi_inv_timeout_count = 0;
            ensure_domain_internal(&mut state, 0);

            let root_phys = state
                .vtd_root_table
                .as_ref()
                .and_then(|root| virt_to_phys_local((&root.0 as *const _ as usize) as usize))
                .unwrap_or(0);
            if root_phys != 0 {
                state.vtd_programmed_units =
                    bootstrap_vtd_hardware(root_phys, &state.dmar_drhd_register_bases);
                state.vtd_hw_ready = state.vtd_programmed_units > 0;
                if state.vtd_hw_ready {
                    vtd_iotlb_global_invalidate(&mut state);
                }
            } else {
                state.vtd_programmed_units = 0;
                state.vtd_hw_ready = false;
                crate::klog_warn!("VT-d root table physical address conversion failed");
            }

            crate::klog_info!(
                "IOMMU backend: VT-d detected dmar={:#x} drhd_units={}",
                info.dmar_addr,
                info.dmar_drhd_units
            );
            for base in &state.dmar_drhd_register_bases {
                crate::klog_info!("VT-d DRHD unit base={:#x}", base);
            }
        } else if info.ivrs_ivhd_units > 0 {
            let mut iommu = AmdVi;
            iommu.init();
            let mut state = IOMMU_STATE.lock();
            state.backend = "amd-vi-hw";
            state.hardware_mode = true;
            state.ivrs_ivhd_register_bases = units.ivrs_ivhd_register_bases;
            state.dmar_drhd_register_bases.clear();
            state.domain_map.clear();
            state.device_domain_map.clear();
            state.vtd_root_table = None;
            state.vtd_context_tables.clear();
            state.amdvi_cmd_ring = None;
            state.amdvi_cmd_tail = 0;
            state.vtd_programmed_units = 0;
            state.vtd_hw_ready = false;
            state.vtd_iotlb_inv_count = 0;
            state.amdvi_inv_count = 0;
            state.amdvi_inv_global_count = 0;
            state.amdvi_inv_domain_count = 0;
            state.amdvi_inv_device_count = 0;
            state.amdvi_inv_fallback_count = 0;
            state.amdvi_inv_timeout_count = 0;
            ensure_domain_internal(&mut state, 0);
            amdvi_issue_global_invalidate(&mut state);
            crate::klog_info!(
                "IOMMU backend: AMD-Vi detected ivrs={:#x} ivhd_units={}",
                info.ivrs_addr,
                info.ivrs_ivhd_units
            );
            for base in &state.ivrs_ivhd_register_bases {
                crate::klog_info!("AMD-Vi IVHD unit base={:#x}", base);
            }
        } else {
            let mut iommu = IntelVtd;
            iommu.init();
            let mut state = IOMMU_STATE.lock();
            state.backend = "intel-vtd-soft";
            state.hardware_mode = false;
            state.dmar_drhd_register_bases.clear();
            state.ivrs_ivhd_register_bases.clear();
            state.domain_map.clear();
            state.device_domain_map.clear();
            state.vtd_root_table = None;
            state.vtd_context_tables.clear();
            state.amdvi_cmd_ring = None;
            state.amdvi_cmd_tail = 0;
            state.vtd_programmed_units = 0;
            state.vtd_hw_ready = false;
            state.vtd_iotlb_inv_count = 0;
            state.amdvi_inv_count = 0;
            state.amdvi_inv_global_count = 0;
            state.amdvi_inv_domain_count = 0;
            state.amdvi_inv_device_count = 0;
            state.amdvi_inv_fallback_count = 0;
            state.amdvi_inv_timeout_count = 0;
            ensure_domain_internal(&mut state, 0);
            crate::klog_warn!("IOMMU hardware tables not found; using software isolation mode");
        }
        IOMMU_INITIALIZED.store(true, Ordering::Relaxed);
    }

    #[cfg(target_arch = "aarch64")]
    {
        let mut iommu = ArmSmmu;
        iommu.init();
        let mut state = IOMMU_STATE.lock();
        state.hardware_mode = false;
        state.domain_map.clear();
        state.device_domain_map.clear();
        state.vtd_root_table = None;
        state.vtd_context_tables.clear();
        state.amdvi_cmd_ring = None;
        state.amdvi_cmd_tail = 0;
        state.vtd_programmed_units = 0;
        state.vtd_hw_ready = false;
        state.vtd_iotlb_inv_count = 0;
        state.amdvi_inv_count = 0;
        state.amdvi_inv_global_count = 0;
        state.amdvi_inv_domain_count = 0;
        state.amdvi_inv_device_count = 0;
        state.amdvi_inv_fallback_count = 0;
        state.amdvi_inv_timeout_count = 0;
        ensure_domain_internal(&mut state, 0);
        IOMMU_INITIALIZED.store(true, Ordering::Relaxed);
        crate::klog_info!("IOMMU init requested (ARM SMMU minimal mode active)");
    }
}

pub fn is_initialized() -> bool {
    IOMMU_INITIALIZED.load(Ordering::Relaxed)
}

pub fn map_dma_page(phys: usize, iova: usize, flags: IommuFlags) -> bool {
    map_page_internal(phys, iova, flags)
}

pub fn unmap_dma_page(iova: usize) -> bool {
    unmap_page_internal(iova)
}

pub fn is_iova_mapped(iova: usize) -> bool {
    IOMMU_STATE.lock().mappings.contains_key(&iova)
}

pub fn iova_mapping(iova: usize) -> Option<(usize, IommuFlags)> {
    IOMMU_STATE
        .lock()
        .mappings
        .get(&iova)
        .copied()
        .map(|m| (m.phys, m.flags))
}

pub fn flush_pending() {
    flush_iotlb_internal();
}

pub fn ensure_domain(domain_id: u16) -> bool {
    if !IOMMU_INITIALIZED.load(Ordering::Relaxed) {
        return false;
    }
    let mut state = IOMMU_STATE.lock();
    ensure_domain_internal(&mut state, domain_id);
    true
}

pub fn attach_device_to_domain(addr: DeviceAddress, domain_id: u16) -> bool {
    if !IOMMU_INITIALIZED.load(Ordering::Relaxed) {
        return false;
    }
    let mut state = IOMMU_STATE.lock();
    let ok = attach_device_internal(&mut state, addr, domain_id);
    if !ok {
        crate::klog_warn!(
            "IOMMU attach rejected: bus={} dev={} fn={} domain={}",
            addr.bus,
            addr.device,
            addr.function,
            domain_id
        );
    }
    ok
}

pub fn device_domain(addr: DeviceAddress) -> Option<u16> {
    if !valid_device_address(addr) {
        return None;
    }
    IOMMU_STATE.lock().device_domain_map.get(&addr.bdf()).copied()
}

pub fn map_dma_page_for_domain(
    domain_id: u16,
    phys: usize,
    iova: usize,
    flags: IommuFlags,
) -> bool {
    if !IOMMU_INITIALIZED.load(Ordering::Relaxed) {
        return false;
    }
    {
        let mut state = IOMMU_STATE.lock();
        ensure_domain_internal(&mut state, domain_id);
    }
    map_page_internal_for_domain(domain_id, phys, iova, flags)
}

pub fn domain_stats(domain_id: u16) -> Option<(usize, usize, usize)> {
    let state = IOMMU_STATE.lock();
    state
        .domain_map
        .get(&domain_id)
        .map(|d| (d.mappings, d.attached_devices, d.slpt_entries))
}

pub fn stats() -> IommuStats {
    let state = IOMMU_STATE.lock();
    IommuStats {
        initialized: IOMMU_INITIALIZED.load(Ordering::Relaxed),
        backend: state.backend,
        hardware_mode: state.hardware_mode,
        vtd_units: state.dmar_drhd_register_bases.len(),
        vtd_programmed_units: state.vtd_programmed_units,
        vtd_hw_ready: state.vtd_hw_ready,
        vtd_iotlb_inv_count: state.vtd_iotlb_inv_count,
        amdvi_units: state.ivrs_ivhd_register_bases.len(),
        amdvi_inv_count: state.amdvi_inv_count,
        amdvi_inv_global_count: state.amdvi_inv_global_count,
        amdvi_inv_domain_count: state.amdvi_inv_domain_count,
        amdvi_inv_device_count: state.amdvi_inv_device_count,
        amdvi_inv_fallback_count: state.amdvi_inv_fallback_count,
        amdvi_inv_timeout_count: state.amdvi_inv_timeout_count,
        domains: state.domain_map.len(),
        attached_devices: state.device_domain_map.len(),
        mapping_count: state.mappings.len(),
        flush_count: state.flush_count,
        map_count: state.map_count,
        unmap_count: state.unmap_count,
    }
}
