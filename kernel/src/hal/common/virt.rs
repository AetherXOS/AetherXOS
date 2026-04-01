mod backend_executor;
mod consts;
mod orchestrator;
mod profiles;
mod runtime_governor;
mod runtime_ops;
#[cfg(test)]
mod tests;
mod virt_ops;

pub use backend_executor::*;
pub use consts::*;
pub use orchestrator::*;
pub use profiles::*;
pub use runtime_governor::*;
pub use runtime_ops::*;
pub use virt_ops::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct VirtCaps {
    pub vmx: bool,
    pub svm: bool,
    pub hypervisor_present: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VirtEnableState {
    pub vmx_enabled: bool,
    pub vmxon_active: bool,
    pub svm_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct VirtStatus {
    pub caps: VirtCaps,
    pub enabled: VirtEnableState,
    pub vm_launch_ready: bool,
    pub blocker: &'static str,
    pub vmx_vmcs_ready: bool,
    pub svm_vmcb_ready: bool,
    pub prep_attempts: u64,
    pub prep_success: u64,
    pub prep_failures: u64,
    pub vmx_lifecycle: &'static str,
    pub svm_lifecycle: &'static str,
}
