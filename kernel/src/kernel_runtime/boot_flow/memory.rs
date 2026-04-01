use crate::kernel_runtime::KernelRuntime;

impl KernelRuntime {
    pub(super) fn init_virtual_memory_runtime(&self) {
        #[cfg(feature = "paging_enable")]
        hypercore::kernel::vmm::init();
    }
}
