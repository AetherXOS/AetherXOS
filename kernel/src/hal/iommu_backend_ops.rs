use super::*;

pub(super) fn vtd_iotlb_global_invalidate(state: &mut IommuState) {
    if state.dmar_drhd_register_bases.is_empty() {
        return;
    }

    for base in &state.dmar_drhd_register_bases {
        let ecap = read_mmio_u64(base + VTD_REG_ECAP).unwrap_or(0);
        let iotlb_reg_offset = ((ecap >> 8) & 0x3ff) * 16;
        if iotlb_reg_offset != 0 {
            let iotlb_reg = base + iotlb_reg_offset;
            let _ = write_mmio_u64(iotlb_reg, VTD_IOTLB_IVT | VTD_IOTLB_IIRG_GLOBAL);
            for _ in 0..IOMMU_MMIO_WAIT_TIMEOUT_SPINS {
                let val = read_mmio_u64(iotlb_reg).unwrap_or(0);
                if (val & VTD_IOTLB_IVT) == 0 {
                    break;
                }
                core::hint::spin_loop();
            }
        } else {
            let _ = write_mmio_u64(base + VTD_REG_CCMD, VTD_CCMD_ICC | VTD_CCMD_CIRG_GLOBAL);
            for _ in 0..IOMMU_MMIO_WAIT_TIMEOUT_SPINS {
                let ccmd = read_mmio_u32(base + VTD_REG_CCMD + 4).unwrap_or(0);
                if (ccmd & 0x8000_0000) == 0 {
                    break;
                }
                core::hint::spin_loop();
            }
        }

        state.vtd_iotlb_inv_count = state.vtd_iotlb_inv_count.saturating_add(1);
    }
}

pub(super) fn amdvi_issue_global_invalidate(state: &mut IommuState) {
    if state.ivrs_ivhd_register_bases.is_empty() {
        return;
    }

    if state.amdvi_cmd_ring.is_none() {
        if !amdvi_setup_command_buffer(state) {
            return;
        }
    }

    if amdvi_emit_global_invalidate(state) {
        state.amdvi_inv_count = state.amdvi_inv_count.saturating_add(1);
        state.amdvi_inv_global_count = state.amdvi_inv_global_count.saturating_add(1);
    }
}

pub(super) fn amdvi_issue_domain_invalidate(state: &mut IommuState, domain_id: u16) {
    if state.ivrs_ivhd_register_bases.is_empty() {
        return;
    }

    if state.amdvi_cmd_ring.is_none() {
        if !amdvi_setup_command_buffer(state) {
            return;
        }
    }

    let domain_ok = amdvi_emit_domain_invalidate(state, domain_id);
    let global_fallback_ok = !domain_ok && amdvi_emit_global_invalidate(state);

    if domain_ok || global_fallback_ok {
        state.amdvi_inv_count = state.amdvi_inv_count.saturating_add(1);
        if domain_ok {
            state.amdvi_inv_domain_count = state.amdvi_inv_domain_count.saturating_add(1);
        } else {
            state.amdvi_inv_global_count = state.amdvi_inv_global_count.saturating_add(1);
            state.amdvi_inv_fallback_count = state.amdvi_inv_fallback_count.saturating_add(1);
        }
    } else {
        crate::klog_warn!(
            "AMD-Vi domain invalidate enqueue failed domain={} (including global fallback)",
            domain_id
        );
    }
}

pub(super) fn amdvi_issue_device_invalidate(state: &mut IommuState, bdf: u16) {
    if state.ivrs_ivhd_register_bases.is_empty() {
        return;
    }

    if state.amdvi_cmd_ring.is_none() {
        if !amdvi_setup_command_buffer(state) {
            return;
        }
    }

    let device_ok = amdvi_emit_device_invalidate(state, bdf);
    let global_fallback_ok = !device_ok && amdvi_emit_global_invalidate(state);

    if device_ok || global_fallback_ok {
        state.amdvi_inv_count = state.amdvi_inv_count.saturating_add(1);
        if device_ok {
            state.amdvi_inv_device_count = state.amdvi_inv_device_count.saturating_add(1);
        } else {
            state.amdvi_inv_global_count = state.amdvi_inv_global_count.saturating_add(1);
            state.amdvi_inv_fallback_count = state.amdvi_inv_fallback_count.saturating_add(1);
        }
    } else {
        crate::klog_warn!(
            "AMD-Vi device invalidate enqueue failed bdf={:#x} (including global fallback)",
            bdf
        );
    }
}

pub(super) fn amdvi_setup_command_buffer(state: &mut IommuState) -> bool {
    if state.ivrs_ivhd_register_bases.is_empty() {
        return false;
    }

    if state.amdvi_cmd_ring.is_none() {
        state.amdvi_cmd_ring = Some(Box::new(AmdViCmdRing::new_zeroed()));
        state.amdvi_cmd_tail = 0;
    }

    let ring_phys = state
        .amdvi_cmd_ring
        .as_ref()
        .and_then(|ring| virt_to_phys_local((&ring.0 as *const _ as usize) as usize))
        .unwrap_or(0);
    if ring_phys == 0 {
        crate::klog_warn!("AMD-Vi command ring physical address conversion failed");
        return false;
    }

    for base in &state.ivrs_ivhd_register_bases {
        let _ = write_mmio_u64(base + AMDVI_CMD_BUFFER_BASE, ring_phys & !0xfff);
        let _ = write_mmio_u32(base + AMDVI_CMD_BUFFER_HEAD, 0);
        let _ = write_mmio_u32(base + AMDVI_CMD_BUFFER_TAIL, 0);
    }

    true
}

pub(super) fn amdvi_ring_would_be_full(state: &IommuState, next_tail: u32) -> bool {
    for base in &state.ivrs_ivhd_register_bases {
        if let Some(head) = read_mmio_u32(base + AMDVI_CMD_BUFFER_HEAD) {
            return head == next_tail;
        }
    }
    false
}

pub(super) fn amdvi_emit_command(state: &mut IommuState, word0: u64, word1: u64) -> bool {
    let Some(ring_words) = state.amdvi_cmd_ring.as_ref().map(|ring| ring.0.len()) else {
        return false;
    };

    let cmd_slots = ring_words / 2;
    if cmd_slots == 0 {
        return false;
    }

    let slot = (state.amdvi_cmd_tail as usize) % cmd_slots;
    let next_tail =
        next_ring_index(state.amdvi_cmd_tail.saturating_mul(2), ring_words).unwrap_or(0) / 2;

    let mut space_available = false;
    for _ in 0..IOMMU_MMIO_WAIT_TIMEOUT_SPINS {
        if !amdvi_ring_would_be_full(state, next_tail) {
            space_available = true;
            break;
        }
        core::hint::spin_loop();
    }

    if !space_available {
        crate::klog_warn!(
            "AMD-Vi command ring full tail={} next_tail={}",
            state.amdvi_cmd_tail,
            next_tail
        );
        return false;
    }

    let Some(ring) = state.amdvi_cmd_ring.as_mut() else {
        return false;
    };

    let idx = slot * 2;
    ring.0[idx] = word0;
    ring.0[idx + 1] = word1;

    state.amdvi_cmd_tail = next_tail;

    let mut any_progress = false;
    for base in &state.ivrs_ivhd_register_bases {
        let _ = write_mmio_u32(base + AMDVI_CMD_BUFFER_TAIL, state.amdvi_cmd_tail);

        let mut completed = false;
        for _ in 0..IOMMU_MMIO_WAIT_TIMEOUT_SPINS {
            let head = read_mmio_u32(base + AMDVI_CMD_BUFFER_HEAD).unwrap_or(0);
            if head == state.amdvi_cmd_tail {
                completed = true;
                break;
            }
            core::hint::spin_loop();
        }

        if completed {
            any_progress = true;
        }
    }

    if !any_progress {
        state.amdvi_inv_timeout_count = state.amdvi_inv_timeout_count.saturating_add(1);
    }

    any_progress
}

pub(super) fn amdvi_emit_global_invalidate(state: &mut IommuState) -> bool {
    amdvi_emit_command(state, AMDVI_INV_CMD_OPCODE_GLOBAL, 0)
}

pub(super) fn amdvi_emit_domain_invalidate(state: &mut IommuState, domain_id: u16) -> bool {
    amdvi_emit_command(
        state,
        AMDVI_INV_CMD_OPCODE_DOMAIN,
        (domain_id as u64) & 0xffff,
    )
}

pub(super) fn amdvi_emit_device_invalidate(state: &mut IommuState, bdf: u16) -> bool {
    amdvi_emit_command(state, AMDVI_INV_CMD_OPCODE_DEVICE, (bdf as u64) & 0xffff)
}

pub(super) fn invalidate_for_backend(state: &mut IommuState) {
    match state.backend {
        "intel-vtd-hw" => vtd_iotlb_global_invalidate(state),
        "amd-vi-hw" => amdvi_issue_global_invalidate(state),
        _ => {}
    }
}

#[allow(dead_code)]
pub(crate) fn bootstrap_vtd_hardware(root_phys: u64, units: &[u64]) -> usize {
    let mut programmed = 0usize;

    for base in units {
        if !write_mmio_u64(base + VTD_REG_RTADDR, root_phys & !0xfff) {
            continue;
        }

        let current_gcmd = read_mmio_u32(base + VTD_REG_GCMD).unwrap_or(0);
        if !write_mmio_u32(base + VTD_REG_GCMD, current_gcmd | VTD_GCMD_SRTP) {
            continue;
        }

        let mut ok = false;
        for _ in 0..IOMMU_MMIO_WAIT_TIMEOUT_SPINS {
            if let Some(gsts) = read_mmio_u32(base + VTD_REG_GSTS) {
                if (gsts & VTD_GSTS_RTPS) != 0 {
                    ok = true;
                    break;
                }
            }
            core::hint::spin_loop();
        }

        if !ok {
            crate::klog_warn!("VT-d SRTP not acknowledged for unit base={:#x}", base);
            continue;
        }

        let _ = write_mmio_u64(base + VTD_REG_CCMD, VTD_CCMD_ICC | VTD_CCMD_CIRG_GLOBAL);
        for _ in 0..IOMMU_MMIO_WAIT_TIMEOUT_SPINS {
            let ccmd = read_mmio_u32(base + VTD_REG_CCMD + 4).unwrap_or(0);
            if (ccmd & 0x8000_0000) == 0 {
                break;
            }
            core::hint::spin_loop();
        }

        programmed = programmed.saturating_add(1);
    }

    programmed
}

