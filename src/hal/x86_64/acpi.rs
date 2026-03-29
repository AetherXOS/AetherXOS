use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, Default)]
pub struct AcpiParserStats {
    pub tables_scanned: usize,
    pub invalid_signatures: usize,
    pub bounds_exceeded: usize,
    pub malformed_entries: usize,
}

struct AcpiParserAtomicStats {
    tables_scanned: AtomicUsize,
    invalid_signatures: AtomicUsize,
    bounds_exceeded: AtomicUsize,
    malformed_entries: AtomicUsize,
}

static ACPI_STATS: AcpiParserAtomicStats = AcpiParserAtomicStats {
    tables_scanned: AtomicUsize::new(0),
    invalid_signatures: AtomicUsize::new(0),
    bounds_exceeded: AtomicUsize::new(0),
    malformed_entries: AtomicUsize::new(0),
};

pub fn parser_stats() -> AcpiParserStats {
    AcpiParserStats {
        tables_scanned: ACPI_STATS.tables_scanned.load(Ordering::Relaxed),
        invalid_signatures: ACPI_STATS.invalid_signatures.load(Ordering::Relaxed),
        bounds_exceeded: ACPI_STATS.bounds_exceeded.load(Ordering::Relaxed),
        malformed_entries: ACPI_STATS.malformed_entries.load(Ordering::Relaxed),
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct AcpiTopology {
    pub rsdp_addr: u64,
    pub madt_addr: u64,
    pub lapic_count: u32,
    pub ioapic_count: u32,
    pub iso_count: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IommuAcpiInfo {
    pub rsdp_addr: u64,
    pub dmar_addr: u64,
    pub dmar_drhd_units: u32,
    pub ivrs_addr: u64,
    pub ivrs_ivhd_units: u32,
}

#[derive(Debug, Default)]
pub struct IommuUnitLists {
    pub dmar_drhd_register_bases: Vec<u64>,
    pub ivrs_ivhd_register_bases: Vec<u64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AcpiPowerInfo {
    pub rsdp_addr: u64,
    pub fadt_addr: u64,
    pub fadt_revision: u8,
    pub has_fadt: bool,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RsdpV1 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RsdpV2 {
    first: RsdpV1,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct SdtHeader {
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

const SDT_HEADER_SIZE: usize = core::mem::size_of::<SdtHeader>();

#[inline(always)]
unsafe fn read_unaligned_copy<T: Copy>(addr: u64) -> T {
    // Safety: callers validate that `addr` points to a readable ACPI structure of type `T`.
    unsafe { core::ptr::read_unaligned(addr as *const T) }
}

fn find_table_from_sdt(sdt_addr: u64, entry_size: usize, signature: [u8; 4]) -> Option<u64> {
    if sdt_addr == 0 {
        return None;
    }

    let header = unsafe { read_unaligned_copy::<SdtHeader>(sdt_addr) };
    let length = header.length as usize;
    if length < SDT_HEADER_SIZE {
        ACPI_STATS.malformed_entries.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    // Hard boundary constraint
    if length > 1024 * 1024 {
        ACPI_STATS.bounds_exceeded.fetch_add(1, Ordering::Relaxed);
        return None;
    }

    let payload_len = length - SDT_HEADER_SIZE;
    let entry_count = payload_len / entry_size;
    let base = sdt_addr as usize + SDT_HEADER_SIZE;

    for index in 0..entry_count {
        let table_addr = if entry_size == 8 {
            unsafe { core::ptr::read_unaligned((base + index * 8) as *const u64) }
        } else {
            unsafe { core::ptr::read_unaligned((base + index * 4) as *const u32) as u64 }
        };

        if table_addr == 0 {
            continue;
        }

        ACPI_STATS.tables_scanned.fetch_add(1, Ordering::Relaxed);
        let table_header = unsafe { read_unaligned_copy::<SdtHeader>(table_addr) };
        if table_header.signature == signature {
            return Some(table_addr);
        } else {
            ACPI_STATS
                .invalid_signatures
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    None
}

fn find_madt(rsdp_addr: u64) -> Option<u64> {
    find_table(rsdp_addr, *b"APIC")
}

pub fn find_table(rsdp_addr: u64, signature: [u8; 4]) -> Option<u64> {
    if rsdp_addr == 0 {
        return None;
    }

    let rsdp_v1 = unsafe { read_unaligned_copy::<RsdpV1>(rsdp_addr) };
    if &rsdp_v1.signature != b"RSD PTR " {
        return None;
    }

    if rsdp_v1.revision >= 2 {
        let rsdp_v2 = unsafe { read_unaligned_copy::<RsdpV2>(rsdp_addr) };
        if rsdp_v2.xsdt_address != 0 {
            return find_table_from_sdt(rsdp_v2.xsdt_address, 8, signature);
        }
    }

    find_table_from_sdt(rsdp_v1.rsdt_address as u64, 4, signature)
}

fn parse_madt_entries(madt_addr: u64) -> AcpiTopology {
    let mut topology = AcpiTopology {
        rsdp_addr: 0,
        madt_addr,
        lapic_count: 0,
        ioapic_count: 0,
        iso_count: 0,
    };

    if madt_addr == 0 {
        return topology;
    }

    let header = unsafe { read_unaligned_copy::<SdtHeader>(madt_addr) };
    let total_len = header.length as usize;
    let madt_fixed_len = SDT_HEADER_SIZE + 8;
    if total_len <= madt_fixed_len {
        return topology;
    }

    let mut offset = madt_fixed_len;
    while offset + 2 <= total_len {
        let entry_base = madt_addr as usize + offset;
        let entry_type = unsafe { core::ptr::read_unaligned(entry_base as *const u8) };
        let entry_len =
            unsafe { core::ptr::read_unaligned((entry_base + 1) as *const u8) } as usize;

        if entry_len < 2 || offset + entry_len > total_len {
            ACPI_STATS.malformed_entries.fetch_add(1, Ordering::Relaxed);
            break;
        }

        match entry_type {
            0 => topology.lapic_count = topology.lapic_count.saturating_add(1),
            1 => topology.ioapic_count = topology.ioapic_count.saturating_add(1),
            2 => topology.iso_count = topology.iso_count.saturating_add(1),
            _ => {}
        }

        offset += entry_len;
    }

    topology
}

pub fn discover_topology() -> AcpiTopology {
    let rsdp_addr = crate::hal::x86_64::acpi_rsdp_addr().unwrap_or(0);
    let madt_addr = find_madt(rsdp_addr).unwrap_or(0);

    let mut topology = parse_madt_entries(madt_addr);
    topology.rsdp_addr = rsdp_addr;
    topology
}

fn count_dmar_drhd_units(dmar_addr: u64) -> u32 {
    if dmar_addr == 0 {
        return 0;
    }

    let header = unsafe { read_unaligned_copy::<SdtHeader>(dmar_addr) };
    let total_len = header.length as usize;
    if total_len <= SDT_HEADER_SIZE + 12 {
        return 0;
    }

    let mut count = 0u32;
    let mut offset = SDT_HEADER_SIZE + 12;
    while offset + 4 <= total_len {
        let base = dmar_addr as usize + offset;
        let entry_type = unsafe { core::ptr::read_unaligned(base as *const u16) };
        let entry_len = unsafe { core::ptr::read_unaligned((base + 2) as *const u16) } as usize;
        if entry_len < 4 || offset + entry_len > total_len {
            ACPI_STATS.malformed_entries.fetch_add(1, Ordering::Relaxed);
            break;
        }
        if entry_type == 0 {
            count = count.saturating_add(1);
        }
        offset += entry_len;
    }

    count
}

fn dmar_drhd_register_bases(dmar_addr: u64) -> Vec<u64> {
    let mut out = Vec::new();
    if dmar_addr == 0 {
        return out;
    }

    let header = unsafe { read_unaligned_copy::<SdtHeader>(dmar_addr) };
    let total_len = header.length as usize;
    if total_len <= SDT_HEADER_SIZE + 12 {
        return out;
    }

    let mut offset = SDT_HEADER_SIZE + 12;
    while offset + 16 <= total_len {
        let base = dmar_addr as usize + offset;
        let entry_type = unsafe { core::ptr::read_unaligned(base as *const u16) };
        let entry_len = unsafe { core::ptr::read_unaligned((base + 2) as *const u16) } as usize;
        if entry_len < 16 || offset + entry_len > total_len {
            ACPI_STATS.malformed_entries.fetch_add(1, Ordering::Relaxed);
            break;
        }
        if entry_type == 0 {
            let register_base = unsafe { core::ptr::read_unaligned((base + 8) as *const u64) };
            if register_base != 0 {
                out.push(register_base);
            }
        }
        offset += entry_len;
    }

    out
}

fn count_ivrs_ivhd_units(ivrs_addr: u64) -> u32 {
    if ivrs_addr == 0 {
        return 0;
    }

    let header = unsafe { read_unaligned_copy::<SdtHeader>(ivrs_addr) };
    let total_len = header.length as usize;
    if total_len <= SDT_HEADER_SIZE + 8 {
        return 0;
    }

    let mut count = 0u32;
    let mut offset = SDT_HEADER_SIZE + 8;
    while offset + 4 <= total_len {
        let base = ivrs_addr as usize + offset;
        let entry_type = unsafe { core::ptr::read_unaligned(base as *const u8) };
        let entry_len = unsafe { core::ptr::read_unaligned((base + 2) as *const u16) } as usize;
        if entry_len < 4 || offset + entry_len > total_len {
            ACPI_STATS.malformed_entries.fetch_add(1, Ordering::Relaxed);
            break;
        }
        if matches!(entry_type, 0x10 | 0x11 | 0x40 | 0x41) {
            count = count.saturating_add(1);
        }
        offset += entry_len;
    }

    count
}

fn ivrs_ivhd_register_bases(ivrs_addr: u64) -> Vec<u64> {
    let mut out = Vec::new();
    if ivrs_addr == 0 {
        return out;
    }

    let header = unsafe { read_unaligned_copy::<SdtHeader>(ivrs_addr) };
    let total_len = header.length as usize;
    if total_len <= SDT_HEADER_SIZE + 8 {
        return out;
    }

    let mut offset = SDT_HEADER_SIZE + 8;
    while offset + 16 <= total_len {
        let base = ivrs_addr as usize + offset;
        let entry_type = unsafe { core::ptr::read_unaligned(base as *const u8) };
        let entry_len = unsafe { core::ptr::read_unaligned((base + 2) as *const u16) } as usize;
        if entry_len < 16 || offset + entry_len > total_len {
            ACPI_STATS.malformed_entries.fetch_add(1, Ordering::Relaxed);
            break;
        }
        if matches!(entry_type, 0x10 | 0x11 | 0x40 | 0x41) {
            let register_base = unsafe { core::ptr::read_unaligned((base + 8) as *const u64) };
            if register_base != 0 {
                out.push(register_base);
            }
        }
        offset += entry_len;
    }

    out
}

pub fn discover_iommu_info() -> IommuAcpiInfo {
    let rsdp_addr = crate::hal::x86_64::acpi_rsdp_addr().unwrap_or(0);
    let dmar_addr = find_table(rsdp_addr, *b"DMAR").unwrap_or(0);
    let ivrs_addr = find_table(rsdp_addr, *b"IVRS").unwrap_or(0);

    IommuAcpiInfo {
        rsdp_addr,
        dmar_addr,
        dmar_drhd_units: count_dmar_drhd_units(dmar_addr),
        ivrs_addr,
        ivrs_ivhd_units: count_ivrs_ivhd_units(ivrs_addr),
    }
}

pub fn discover_iommu_units() -> IommuUnitLists {
    let rsdp_addr = crate::hal::x86_64::acpi_rsdp_addr().unwrap_or(0);
    let dmar_addr = find_table(rsdp_addr, *b"DMAR").unwrap_or(0);
    let ivrs_addr = find_table(rsdp_addr, *b"IVRS").unwrap_or(0);

    IommuUnitLists {
        dmar_drhd_register_bases: dmar_drhd_register_bases(dmar_addr),
        ivrs_ivhd_register_bases: ivrs_ivhd_register_bases(ivrs_addr),
    }
}

pub fn discover_power_info() -> AcpiPowerInfo {
    let rsdp_addr = crate::hal::x86_64::acpi_rsdp_addr().unwrap_or(0);
    let fadt_addr = find_table(rsdp_addr, *b"FACP").unwrap_or(0);

    if fadt_addr == 0 {
        return AcpiPowerInfo {
            rsdp_addr,
            fadt_addr: 0,
            fadt_revision: 0,
            has_fadt: false,
        };
    }

    let header = unsafe { read_unaligned_copy::<SdtHeader>(fadt_addr) };
    AcpiPowerInfo {
        rsdp_addr,
        fadt_addr,
        fadt_revision: header.revision,
        has_fadt: true,
    }
}
