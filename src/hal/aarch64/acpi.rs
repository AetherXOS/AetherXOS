/// AArch64 ACPI / DTB discovery helpers.
///
/// On AArch64 systems, platform topology is described either via:
///   * ACPI RSDP → XSDT → MADT (GIC, GICD, GICR, MPS interrupt source overrides)
///   * Flattened Device Tree (FDT / DTB)
///
/// This module implements real table walks where the tables are memory-mapped
/// by the bootloader (Limine or UEFI entry path).
use core::sync::atomic::{AtomicUsize, Ordering};

// ── Public types ──────────────────────────────────────────────────────────────

pub struct AcpiTopologyInfo {
    pub rsdp_addr: u64,
    pub lapic_count: usize,
    pub ioapic_count: usize,
    pub iso_count: usize,
}

pub struct AcpiPowerInfo {
    pub rsdp_addr: u64,
    pub fadt_addr: u64,
    pub has_fadt: bool,
    pub fadt_revision: u8,
}

// ── Internal ACPI structures ──────────────────────────────────────────────────

/// Standard ACPI System Description Table header (36 bytes).
#[repr(C, packed)]
struct AcpiSdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

/// ACPI RSDP (v2+) — 36 bytes.
#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    // v2+ fields:
    length: u32,
    xsdt_address: u64,
    ext_checksum: u8,
    _reserved: [u8; 3],
}

/// MADT sub-entry type 11: GICC (Generic Interrupt Controller CPU Interface).
#[repr(C, packed)]
struct MadtGicc {
    typ: u8, // 11
    length: u8,
    _reserved: u16,
    cpu_interface_num: u32,
    acpi_uid: u32,
    flags: u32,
    parking_version: u32,
    perf_gsi: u32,
    parked_address: u64,
    base_address: u64,
    gicv_base: u64,
    gich_base: u64,
    vgic_maint_irq: u32,
    gicr_base: u64,
    mpidr: u64,
    processor_pwr_eff: u8,
    _reserved2: u8,
    spe_overflow_irq: u16,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Validate an ACPI table checksum.
unsafe fn valid_checksum(base: *const u8, len: usize) -> bool {
    let mut sum: u8 = 0;
    for i in 0..len {
        sum = sum.wrapping_add(*base.add(i));
    }
    sum == 0
}

/// Convert a physical address to a virtual pointer using the HHDM offset.
fn phys_to_virt(phys: u64) -> Option<*const u8> {
    let hhdm = crate::hal::hhdm_offset()?;
    Some((phys + hhdm) as *const u8)
}

// ── MADT walk ────────────────────────────────────────────────────────────────

#[derive(Default)]
struct MadtTopology {
    gicc_count: usize,
    gicd_count: usize,
    gicr_count: usize,
    iso_count: usize,
}

unsafe fn walk_madt(madt_phys: u64) -> MadtTopology {
    let mut topo = MadtTopology::default();
    let Some(madt_ptr) = phys_to_virt(madt_phys) else {
        return topo;
    };

    let hdr = &*(madt_ptr as *const AcpiSdtHeader);
    let table_len = { hdr.length } as usize;
    if !valid_checksum(madt_ptr, table_len) {
        crate::klog_warn!("ACPI MADT checksum invalid");
        return topo;
    }

    // MADT-specific header is 8 bytes after the common SDT header (44 bytes total before entries).
    let mut offset: usize = 44;
    while offset + 2 <= table_len {
        let entry_type = *madt_ptr.add(offset);
        let entry_len = *madt_ptr.add(offset + 1) as usize;
        if entry_len < 2 || offset + entry_len > table_len {
            break;
        }

        match entry_type {
            // Type 0: Local APIC (not typical on AArch64 but may appear in hybrid firmware)
            0 => { /* skip */ }
            // Type 11: GICC
            11 => {
                topo.gicc_count += 1;
                if entry_len >= core::mem::size_of::<MadtGicc>() {
                    let gicc = &*(madt_ptr.add(offset) as *const MadtGicc);
                    let mpidr = { gicc.mpidr };
                    let base = { gicc.base_address };
                    crate::klog_debug!("MADT GICC: mpidr={:#x} base={:#x}", mpidr, base);
                }
            }
            // Type 12: GICD
            12 => {
                topo.gicd_count += 1;
            }
            // Type 14: GICR
            14 => {
                topo.gicr_count += 1;
            }
            // Type  2: ISO (Interrupt Source Override)
            2 => {
                topo.iso_count += 1;
            }
            _ => { /* unknown entry type */ }
        }
        offset += entry_len;
    }
    topo
}

// ── XSDT walk ────────────────────────────────────────────────────────────────

unsafe fn find_table_in_xsdt(xsdt_phys: u64, sig: &[u8; 4]) -> Option<u64> {
    let Some(xsdt_ptr) = phys_to_virt(xsdt_phys) else {
        return None;
    };
    let hdr = &*(xsdt_ptr as *const AcpiSdtHeader);
    let table_len = { hdr.length } as usize;
    if !valid_checksum(xsdt_ptr, table_len) {
        return None;
    }

    // XSDT entries: 8-byte physical addresses starting at offset 36.
    let n_entries = (table_len - 36) / 8;
    for i in 0..n_entries {
        let entry_ptr = (xsdt_ptr as usize + 36 + i * 8) as *const u64;
        let entry_phys = core::ptr::read_unaligned(entry_ptr);
        let Some(tbl_ptr) = phys_to_virt(entry_phys) else {
            continue;
        };
        let tbl_hdr = &*(tbl_ptr as *const AcpiSdtHeader);
        if &tbl_hdr.signature == sig {
            return Some(entry_phys);
        }
    }
    None
}

// ── Statistics ────────────────────────────────────────────────────────────────

static ACPI_PARSE_CALLS: AtomicUsize = AtomicUsize::new(0);

pub fn acpi_parse_calls() -> usize {
    ACPI_PARSE_CALLS.load(Ordering::Relaxed)
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn discover_topology() -> AcpiTopologyInfo {
    ACPI_PARSE_CALLS.fetch_add(1, Ordering::Relaxed);

    let rsdp_phys = super::acpi_rsdp_addr().unwrap_or(0);
    let dtb_phys = super::dtb_addr().unwrap_or(0);

    let mut info = AcpiTopologyInfo {
        rsdp_addr: rsdp_phys,
        lapic_count: 1,
        ioapic_count: 0,
        iso_count: 0,
    };

    if rsdp_phys != 0 {
        crate::klog_info!("AArch64 ACPI RSDP at {:#x}", rsdp_phys);

        if let Some(rsdp_ptr) = phys_to_virt(rsdp_phys) {
            let rsdp = unsafe { &*(rsdp_ptr as *const Rsdp) };
            let xsdt_phys =
                unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(rsdp.xsdt_address)) };

            if xsdt_phys != 0 {
                crate::klog_debug!("XSDT at {:#x}", xsdt_phys);
                if let Some(madt_phys) = unsafe { find_table_in_xsdt(xsdt_phys, b"APIC") } {
                    crate::klog_debug!("MADT at {:#x}", madt_phys);
                    let topo = unsafe { walk_madt(madt_phys) };
                    info.lapic_count = topo.gicc_count.max(1);
                    info.ioapic_count = topo.gicd_count;
                    info.iso_count = topo.iso_count;
                }
            }
        }
    } else if dtb_phys != 0 {
        // DTB-based topology: real DTB parsing would use a crate like `fdt`.
        // For now, log and use safe defaults.
        crate::klog_info!(
            "AArch64 DTB at {:#x} (topology from DTB not yet parsed)",
            dtb_phys
        );
    } else {
        crate::klog_warn!("AArch64: neither ACPI RSDP nor DTB found — single-core assumed");
    }

    info
}

pub fn discover_power_info() -> AcpiPowerInfo {
    ACPI_PARSE_CALLS.fetch_add(1, Ordering::Relaxed);

    let rsdp_phys = super::acpi_rsdp_addr().unwrap_or(0);
    let mut power = AcpiPowerInfo {
        rsdp_addr: rsdp_phys,
        fadt_addr: 0,
        has_fadt: false,
        fadt_revision: 0,
    };

    if rsdp_phys != 0 {
        if let Some(rsdp_ptr) = phys_to_virt(rsdp_phys) {
            let rsdp = unsafe { &*(rsdp_ptr as *const Rsdp) };
            let xsdt_phys =
                unsafe { core::ptr::read_unaligned(core::ptr::addr_of!(rsdp.xsdt_address)) };

            if xsdt_phys != 0 {
                if let Some(fadt_phys) = unsafe { find_table_in_xsdt(xsdt_phys, b"FACP") } {
                    let Some(fadt_ptr) = phys_to_virt(fadt_phys) else {
                        return power;
                    };
                    let hdr = unsafe { &*(fadt_ptr as *const AcpiSdtHeader) };
                    power.fadt_addr = fadt_phys;
                    power.has_fadt = true;
                    power.fadt_revision = hdr.revision;
                    crate::klog_debug!("FADT at {:#x} rev={}", fadt_phys, hdr.revision);
                }
            }
        }
    }

    power
}
