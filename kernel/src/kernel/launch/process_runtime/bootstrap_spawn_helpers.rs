use super::*;
use alloc::sync::Arc;
use crate::kernel::process::Process;

#[cfg(feature = "paging_enable")]
pub fn create_process_with_cr3(name: &[u8]) -> (Arc<Process>, x86_64::PhysAddr) {
    let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
    let offset = x86_64::VirtAddr::new(hhdm);
    let _active_lvl4 = crate::kernel::memory::paging::active_level_4_table(offset.as_u64());
    #[cfg(target_os = "none")]
    {
        let cr3 = x86_64::registers::control::Cr3::read().0.start_address();
        let process = Arc::new(Process::new_with_cr3(name, cr3));
        (process, cr3)
    }
    #[cfg(not(target_os = "none"))]
    {
        let cr3 = x86_64::PhysAddr::new(0);
        let process = Arc::new(Process::new_with_cr3(name, cr3));
        (process, cr3)
    }
}

#[cfg(not(feature = "paging_enable"))]
pub fn create_process_with_cr3(name: &[u8]) -> (Arc<Process>, x86_64::PhysAddr) {
    let process = Arc::new(Process::new(name));
    let cr3 = x86_64::PhysAddr::new(0);
    (process, cr3)
}
