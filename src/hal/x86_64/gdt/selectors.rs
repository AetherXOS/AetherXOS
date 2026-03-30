use x86_64::structures::gdt::SegmentSelector;

/// Selectors used for Kernel/User transitions.
#[derive(Debug, Clone, Copy)]
pub struct Selectors {
    pub kernel_code_selector: SegmentSelector,
    pub kernel_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

impl Selectors {
    pub const fn new_null() -> Self {
        Self {
            kernel_code_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
            kernel_data_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
            user_code_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
            user_data_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
            tss_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
        }
    }
}
