use super::{DeviceAddress, IommuFlags};
use crate::hal::common::mmio::{
    read_phys_u32, read_phys_u64, virt_to_phys, write_phys_u32, write_phys_u64,
};

pub(super) const PAGE_SIZE: usize = 4096;

#[inline(always)]
pub(super) fn valid_device_address(addr: DeviceAddress) -> bool {
    addr.device < 32 && addr.function < 8
}

#[inline(always)]
pub(super) fn is_page_aligned(addr: usize) -> bool {
    addr & (PAGE_SIZE - 1) == 0
}

#[inline(always)]
pub(super) fn can_map_page(phys: usize, iova: usize, flags: IommuFlags) -> bool {
    flags.bits() != 0 && is_page_aligned(phys) && is_page_aligned(iova)
}

#[inline(always)]
pub(super) fn next_ring_index(current_tail: u32, ring_words: usize) -> Option<u32> {
    let words = u32::try_from(ring_words).ok()?;
    if words == 0 {
        return None;
    }
    Some((current_tail + 2) % words)
}

pub(super) fn read_mmio_u32(phys: u64) -> Option<u32> {
    read_phys_u32(phys)
}

#[inline(always)]
pub(super) fn write_mmio_u32(phys: u64, value: u32) -> bool {
    write_phys_u32(phys, value)
}

#[inline(always)]
pub(super) fn write_mmio_u64(phys: u64, value: u64) -> bool {
    write_phys_u64(phys, value)
}

#[inline(always)]
pub(super) fn read_mmio_u64(phys: u64) -> Option<u64> {
    read_phys_u64(phys)
}

#[inline(always)]
pub(super) fn virt_to_phys_local(addr: usize) -> Option<u64> {
    virt_to_phys(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn device_address_validation_matches_pci_bounds() {
        assert!(valid_device_address(DeviceAddress {
            bus: 0,
            device: 31,
            function: 7
        }));
        assert!(!valid_device_address(DeviceAddress {
            bus: 0,
            device: 32,
            function: 0
        }));
        assert!(!valid_device_address(DeviceAddress {
            bus: 0,
            device: 0,
            function: 8
        }));
    }

    #[test_case]
    fn page_map_guard_requires_alignment_and_permissions() {
        let rw = IommuFlags::READ | IommuFlags::WRITE;
        assert!(can_map_page(0x2000, 0x4000, rw));
        assert!(!can_map_page(0x2001, 0x4000, rw));
        assert!(!can_map_page(0x2000, 0x4001, rw));
        assert!(!can_map_page(0x2000, 0x4000, IommuFlags::empty()));
    }

    #[test_case]
    fn ring_index_wraps_in_command_pairs() {
        assert_eq!(next_ring_index(0, 8), Some(2));
        assert_eq!(next_ring_index(6, 8), Some(0));
        assert_eq!(next_ring_index(1, 0), None);
    }
}
