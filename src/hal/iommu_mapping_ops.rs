use super::*;

pub(super) fn ensure_domain_internal(state: &mut IommuState, domain_id: u16) {
    state.domain_map.entry(domain_id).or_default();

    if state.backend == "intel-vtd-hw" {
        if let Some(domain) = state.domain_map.get_mut(&domain_id) {
            if domain.slpt_root.is_none() {
                domain.slpt_root = Some(Box::new(VtdPage::new_zeroed()));
                domain.slpt_phys = domain
                    .slpt_root
                    .as_ref()
                    .and_then(|root| virt_to_phys_local((&root.0 as *const _ as usize) as usize))
                    .unwrap_or(0);
            }
        }
    }
}

fn ensure_vtd_context_table_for_bus(state: &mut IommuState, bus: u8) -> Option<u64> {
    if !state.vtd_context_tables.contains_key(&bus) {
        state
            .vtd_context_tables
            .insert(bus, Box::new(VtdPage::new_zeroed()));
    }

    let context_phys = {
        let table = state.vtd_context_tables.get(&bus)?;
        virt_to_phys_local((&table.0 as *const _ as usize) as usize)?
    };

    if let Some(root) = state.vtd_root_table.as_mut() {
        root.0[bus as usize] = (context_phys & !0xfff) | 1;
    }

    Some(context_phys)
}

fn set_vtd_context_entry(state: &mut IommuState, addr: DeviceAddress, domain_id: u16) -> bool {
    let Some(_ctx_phys) = ensure_vtd_context_table_for_bus(state, addr.bus) else {
        return false;
    };

    let Some(domain) = state.domain_map.get(&domain_id) else {
        return false;
    };
    if domain.slpt_phys == 0 {
        return false;
    }

    let Some(ctx_table) = state.vtd_context_tables.get_mut(&addr.bus) else {
        return false;
    };

    let devfn = ((addr.device as usize) << 3) | (addr.function as usize);
    let idx = devfn * 2;
    if idx + 1 >= 512 {
        return false;
    }

    let low = (domain.slpt_phys & !0xfff) | VTD_CTX_TRANSLATION_TYPE_0 | VTD_CTX_PRESENT;
    let high = domain_id as u64;
    ctx_table.0[idx] = low;
    ctx_table.0[idx + 1] = high;
    true
}

fn vtd_slpt_flags_from_iommu(flags: IommuFlags) -> u64 {
    let mut out = 0u64;
    if flags.contains(IommuFlags::READ) {
        out |= VTD_SLPT_READ;
    }
    if flags.contains(IommuFlags::WRITE) {
        out |= VTD_SLPT_WRITE;
    }
    if flags.contains(IommuFlags::EXECUTE) {
        out |= VTD_SLPT_EXEC;
    }
    out
}

fn set_vtd_second_level_entry(
    state: &mut IommuState,
    domain_id: u16,
    iova: usize,
    phys: usize,
    flags: IommuFlags,
) -> bool {
    if !state.vtd_hw_ready {
        return true;
    }

    let Some(domain) = state.domain_map.get_mut(&domain_id) else {
        return false;
    };
    if domain.slpt_root.is_none() {
        return false;
    }

    let l2_index = (iova >> 21) & 0x1ff;
    let l1_index = (iova >> 12) & 0x1ff;

    if !domain.slpt_leaf_tables.contains_key(&l2_index) {
        domain
            .slpt_leaf_tables
            .insert(l2_index, Box::new(VtdPage::new_zeroed()));

        let leaf_phys = domain
            .slpt_leaf_tables
            .get(&l2_index)
            .and_then(|leaf| virt_to_phys_local((&leaf.0 as *const _ as usize) as usize))
            .unwrap_or(0);
        if leaf_phys == 0 {
            return false;
        }

        if let Some(root) = domain.slpt_root.as_mut() {
            root.0[l2_index] =
                (leaf_phys & !0xfff) | VTD_SLPT_READ | VTD_SLPT_WRITE | VTD_SLPT_EXEC;
        }
    }

    let entry_flags = vtd_slpt_flags_from_iommu(flags);
    if entry_flags == 0 {
        return false;
    }

    let entry = ((phys as u64) & !0xfff) | entry_flags;
    let Some(leaf) = domain.slpt_leaf_tables.get_mut(&l2_index) else {
        return false;
    };

    if leaf.0[l1_index] == 0 {
        domain.slpt_entries = domain.slpt_entries.saturating_add(1);
    }
    leaf.0[l1_index] = entry;
    true
}

fn clear_vtd_second_level_entry(state: &mut IommuState, domain_id: u16, iova: usize) {
    if !state.vtd_hw_ready {
        return;
    }
    let Some(domain) = state.domain_map.get_mut(&domain_id) else {
        return;
    };
    let l2_index = (iova >> 21) & 0x1ff;
    let l1_index = (iova >> 12) & 0x1ff;

    let Some(leaf) = domain.slpt_leaf_tables.get_mut(&l2_index) else {
        return;
    };

    if leaf.0[l1_index] != 0 {
        leaf.0[l1_index] = 0;
        domain.slpt_entries = domain.slpt_entries.saturating_sub(1);
    }

    let leaf_empty = leaf.0.iter().all(|entry| *entry == 0);
    if leaf_empty {
        domain.slpt_leaf_tables.remove(&l2_index);
        if let Some(root) = domain.slpt_root.as_mut() {
            root.0[l2_index] = 0;
        }
    }
}

pub(super) fn attach_device_internal(
    state: &mut IommuState,
    addr: DeviceAddress,
    domain_id: u16,
) -> bool {
    if !valid_device_address(addr) {
        return false;
    }
    ensure_domain_internal(state, domain_id);

    let bdf = addr.bdf();
    if let Some(old_domain) = state.device_domain_map.insert(bdf, domain_id) {
        if let Some(old) = state.domain_map.get_mut(&old_domain) {
            old.attached_devices = old.attached_devices.saturating_sub(1);
        }
    }

    if let Some(new_domain) = state.domain_map.get_mut(&domain_id) {
        new_domain.attached_devices = new_domain.attached_devices.saturating_add(1);
    }

    if state.backend == "intel-vtd-hw" {
        let _ = ensure_vtd_context_table_for_bus(state, addr.bus);
        if !set_vtd_context_entry(state, addr, domain_id) {
            crate::klog_warn!(
                "VT-d context entry write failed bus={} dev={} fn={} domain={}",
                addr.bus,
                addr.device,
                addr.function,
                domain_id
            );
            return false;
        }
        invalidate_for_backend(state);
    } else if state.backend == "amd-vi-hw" {
        amdvi_issue_device_invalidate(state, bdf);
    }
    true
}

pub(super) fn map_page_internal_for_domain(
    domain_id: u16,
    phys: usize,
    iova: usize,
    flags: IommuFlags,
) -> bool {
    if !IOMMU_INITIALIZED.load(Ordering::Relaxed) {
        crate::klog_warn!("IOMMU map ignored: IOMMU not initialized");
        return false;
    }
    if !can_map_page(phys, iova, flags) {
        if flags.is_empty() {
            crate::klog_warn!(
                "IOMMU map ignored: empty permissions phys={:#x} iova={:#x}",
                phys,
                iova
            );
        } else {
            crate::klog_warn!(
                "IOMMU map ignored: unaligned phys={:#x} iova={:#x}",
                phys,
                iova
            );
        }
        return false;
    }

    let mut state = IOMMU_STATE.lock();
    if state.mappings.contains_key(&iova) {
        crate::klog_warn!("IOMMU map ignored: iova already mapped iova={:#x}", iova);
        return false;
    }

    state.mappings.insert(
        iova,
        Mapping {
            domain_id,
            phys,
            flags,
        },
    );
    if state.backend == "intel-vtd-hw" {
        if !set_vtd_second_level_entry(&mut state, domain_id, iova, phys, flags) {
            state.mappings.remove(&iova);
            crate::klog_warn!(
                "VT-d SLPT entry write failed: domain={} iova={:#x} phys={:#x}",
                domain_id,
                iova,
                phys
            );
            return false;
        }
    }
    state.map_count = state.map_count.saturating_add(1);
    if let Some(domain) = state.domain_map.get_mut(&domain_id) {
        domain.mappings = domain.mappings.saturating_add(1);
    }
    if state.backend == "amd-vi-hw" {
        amdvi_issue_domain_invalidate(&mut state, domain_id);
    } else {
        invalidate_for_backend(&mut state);
    }
    true
}

pub(super) fn map_page_internal(phys: usize, iova: usize, flags: IommuFlags) -> bool {
    map_page_internal_for_domain(0, phys, iova, flags)
}

pub(super) fn unmap_page_internal(iova: usize) -> bool {
    if !IOMMU_INITIALIZED.load(Ordering::Relaxed) {
        crate::klog_warn!("IOMMU unmap ignored: IOMMU not initialized");
        return false;
    }
    if !is_page_aligned(iova) {
        crate::klog_warn!("IOMMU unmap ignored: unaligned iova={:#x}", iova);
        return false;
    }

    let mut state = IOMMU_STATE.lock();
    if let Some(mapping) = state.mappings.remove(&iova) {
        if state.backend == "intel-vtd-hw" {
            clear_vtd_second_level_entry(&mut state, mapping.domain_id, iova);
        }
        state.unmap_count = state.unmap_count.saturating_add(1);
        if let Some(domain) = state.domain_map.get_mut(&mapping.domain_id) {
            domain.mappings = domain.mappings.saturating_sub(1);
        }
        if state.backend == "amd-vi-hw" {
            amdvi_issue_domain_invalidate(&mut state, mapping.domain_id);
        } else {
            invalidate_for_backend(&mut state);
        }
        true
    } else {
        crate::klog_warn!("IOMMU unmap ignored: iova not mapped iova={:#x}", iova);
        false
    }
}

pub(super) fn flush_iotlb_internal() {
    if !IOMMU_INITIALIZED.load(Ordering::Relaxed) {
        return;
    }
    let mut state = IOMMU_STATE.lock();
    state.flush_count = state.flush_count.saturating_add(1);
    invalidate_for_backend(&mut state);
}