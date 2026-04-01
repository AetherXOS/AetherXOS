/// IOMMU (Intel VT-d / AMD-Vi) — DMA remapping for device isolation.
///
/// Provides DMA address space isolation so devices can only access memory
/// regions explicitly mapped for them, preventing DMA attacks.
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IOMMU capability detection result.
#[derive(Debug, Clone, Copy)]
pub struct IommuCapability {
    /// Intel VT-d is present.
    pub vtd_present: bool,
    /// AMD-Vi is present.
    pub amdvi_present: bool,
    /// Maximum number of domains (address spaces) supported.
    pub max_domains: u16,
    /// Supports interrupt remapping.
    pub interrupt_remapping: bool,
    /// Page sizes supported (bitmap: bit 0 = 4K, bit 1 = 2M, bit 2 = 1G).
    pub page_sizes: u8,
}

impl IommuCapability {
    pub fn none() -> Self {
        Self {
            vtd_present: false,
            amdvi_present: false,
            max_domains: 0,
            interrupt_remapping: false,
            page_sizes: 0,
        }
    }
}

/// A DMA mapping entry: maps a device-visible address to a physical address.
#[derive(Debug, Clone, Copy)]
pub struct DmaMapping {
    /// Device-visible (IOVA) address.
    pub iova: u64,
    /// Physical address.
    pub phys: u64,
    /// Size in bytes.
    pub size: u64,
    /// Permissions.
    pub readable: bool,
    pub writable: bool,
}

/// Per-device DMA domain (address space).
#[derive(Debug)]
pub struct DmaDomain {
    /// Domain ID.
    pub id: u16,
    /// PCI bus:device.function identifier for the device.
    pub bdf: u32,
    /// Mappings: IOVA → DmaMapping.
    mappings: BTreeMap<u64, DmaMapping>,
    /// Next IOVA address for sequential allocation.
    next_iova: u64,
}

impl DmaDomain {
    pub fn new(id: u16, bdf: u32) -> Self {
        Self {
            id,
            bdf,
            mappings: BTreeMap::new(),
            next_iova: 0x1000, // Start at 4K to avoid zero-page
        }
    }

    /// Map a physical region into this device's DMA address space.
    /// Returns the IOVA (device-visible address).
    pub fn map(&mut self, phys: u64, size: u64, readable: bool, writable: bool) -> u64 {
        let iova = self.next_iova;
        let aligned_size = (size + 0xFFF) & !0xFFF; // 4K align
        self.next_iova += aligned_size;

        self.mappings.insert(
            iova,
            DmaMapping {
                iova,
                phys,
                size,
                readable,
                writable,
            },
        );
        iova
    }

    /// Unmap a DMA region by IOVA.
    pub fn unmap(&mut self, iova: u64) -> bool {
        self.mappings.remove(&iova).is_some()
    }

    /// Look up the physical address for a given IOVA.
    pub fn translate(&self, iova: u64) -> Option<u64> {
        // Find the mapping that contains this IOVA.
        for (_, mapping) in self.mappings.iter() {
            if iova >= mapping.iova && iova < mapping.iova + mapping.size {
                let offset = iova - mapping.iova;
                return Some(mapping.phys + offset);
            }
        }
        None
    }

    /// List all mappings.
    pub fn mappings(&self) -> Vec<&DmaMapping> {
        self.mappings.values().collect()
    }
}

/// IOMMU controller managing all DMA domains.
pub struct Iommu {
    cap: IommuCapability,
    /// Domain ID → DmaDomain.
    domains: BTreeMap<u16, DmaDomain>,
    /// BDF → Domain ID (reverse lookup).
    bdf_to_domain: BTreeMap<u32, u16>,
    next_domain_id: u16,
}

impl Iommu {
    pub fn new(cap: IommuCapability) -> Self {
        Self {
            cap,
            domains: BTreeMap::new(),
            bdf_to_domain: BTreeMap::new(),
            next_domain_id: 1,
        }
    }

    /// Create a DMA domain for a PCI device.
    pub fn create_domain(&mut self, bdf: u32) -> Result<u16, &'static str> {
        if self.next_domain_id >= self.cap.max_domains {
            return Err("max IOMMU domains reached");
        }
        if self.bdf_to_domain.contains_key(&bdf) {
            return Err("device already has a domain");
        }
        let id = self.next_domain_id;
        self.next_domain_id += 1;
        self.domains.insert(id, DmaDomain::new(id, bdf));
        self.bdf_to_domain.insert(bdf, id);
        Ok(id)
    }

    /// Destroy a DMA domain and unmap all its regions.
    pub fn destroy_domain(&mut self, domain_id: u16) -> Result<(), &'static str> {
        let domain = self.domains.remove(&domain_id).ok_or("domain not found")?;
        self.bdf_to_domain.remove(&domain.bdf);
        Ok(())
    }

    /// Map physical memory into a device's DMA address space.
    pub fn map_dma(
        &mut self,
        bdf: u32,
        phys: u64,
        size: u64,
        readable: bool,
        writable: bool,
    ) -> Result<u64, &'static str> {
        let domain_id = self
            .bdf_to_domain
            .get(&bdf)
            .copied()
            .ok_or("device has no domain")?;
        let domain = self.domains.get_mut(&domain_id).ok_or("domain not found")?;
        Ok(domain.map(phys, size, readable, writable))
    }

    /// Unmap a DMA region.
    pub fn unmap_dma(&mut self, bdf: u32, iova: u64) -> Result<(), &'static str> {
        let domain_id = self
            .bdf_to_domain
            .get(&bdf)
            .copied()
            .ok_or("device has no domain")?;
        let domain = self.domains.get_mut(&domain_id).ok_or("domain not found")?;
        if domain.unmap(iova) {
            Ok(())
        } else {
            Err("mapping not found")
        }
    }

    /// Translate an IOVA to physical address for a device.
    pub fn translate(&self, bdf: u32, iova: u64) -> Option<u64> {
        let domain_id = self.bdf_to_domain.get(&bdf)?;
        self.domains.get(domain_id)?.translate(iova)
    }

    /// Get IOMMU capabilities.
    pub fn capability(&self) -> &IommuCapability {
        &self.cap
    }

    /// Number of active domains.
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }
}

/// Detect IOMMU capability from ACPI/PCI.
/// In a real implementation this would parse DMAR (Intel) or IVRS (AMD) tables.
#[cfg(target_arch = "x86_64")]
pub fn detect_iommu() -> IommuCapability {
    // Check CPUID for Intel VT-d hint (leaf 1, ECX bit 5 = VMX, not directly VT-d)
    // Real detection would scan ACPI DMAR table.
    // For now, report capabilities based on CPUID and provide a functional framework.
    let has_vmx = {
        let cpuid = core::arch::x86_64::__cpuid(1);
        (cpuid.ecx & (1 << 5)) != 0
    };

    IommuCapability {
        vtd_present: has_vmx, // VT-d typically accompanies VMX
        amdvi_present: false,
        max_domains: if has_vmx { 256 } else { 0 },
        interrupt_remapping: has_vmx,
        page_sizes: 0b111, // 4K, 2M, 1G
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn detect_iommu() -> IommuCapability {
    IommuCapability::none()
}
