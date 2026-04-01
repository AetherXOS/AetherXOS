#[derive(Debug, Clone, Copy)]
pub struct VirtualizationContractReport {
    pub checks: u32,
    pub failures: u32,
    pub last_error_code: u32,
}

impl VirtualizationContractReport {
    #[inline(always)]
    pub const fn passed(self) -> bool {
        self.failures == 0
    }
}
