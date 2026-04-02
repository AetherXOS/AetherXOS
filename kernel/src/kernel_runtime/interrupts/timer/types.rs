pub(super) struct SwitchInfo {
    pub(super) next_sp: usize,
    pub(super) current_sp_ptr: *mut usize,
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    pub(super) next_tls: u64,
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    pub(super) next_cr3: u64,
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    pub(super) next_kernel_sp: usize,
}
