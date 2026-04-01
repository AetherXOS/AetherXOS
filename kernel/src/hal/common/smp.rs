#[derive(Debug, Clone, Copy)]
pub struct SmpWaitStats {
    pub boot_timeout_spins: usize,
    pub boot_timeouts: u64,
    pub tlb_shootdown_timeout_spins: usize,
    pub tlb_shootdown_timeouts: u64,
}

#[inline(always)]
pub fn wait_stats(
    boot_timeout_spins: usize,
    boot_timeouts: u64,
    tlb_shootdown_timeout_spins: usize,
    tlb_shootdown_timeouts: u64,
) -> SmpWaitStats {
    SmpWaitStats {
        boot_timeout_spins,
        boot_timeouts,
        tlb_shootdown_timeout_spins,
        tlb_shootdown_timeouts,
    }
}
