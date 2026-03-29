#[cfg(all(feature = "vfs", feature = "posix_mman"))]
use super::*;

#[cfg(all(feature = "vfs", feature = "posix_mman"))]
fn fallback_map_id_from_addr(addr: usize) -> u32 {
    addr as u32
}

#[cfg(all(feature = "vfs", feature = "posix_mman"))]
pub(super) fn resolve_map_id_from_addr(addr: usize) -> u32 {
    if let Some(pid) = current_process_id() {
        if let Some(process) =
            crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
        {
            if let Some(mapping) = process.lookup_mapping(addr as u64) {
                return mapping.map_id;
            }
        }
    }

    fallback_map_id_from_addr(addr)
}

#[cfg(all(test, feature = "vfs", feature = "posix_mman"))]
mod tests {
    use super::*;

    #[test_case]
    fn fallback_map_id_preserves_low_u32_bits_of_address() {
        assert_eq!(fallback_map_id_from_addr(0x1234usize), 0x1234);
        assert_eq!(fallback_map_id_from_addr(usize::MAX), u32::MAX);
    }

    #[test_case]
    fn resolve_map_id_falls_back_to_address_without_process_context() {
        assert_eq!(resolve_map_id_from_addr(0x4321usize), 0x4321);
    }
}
