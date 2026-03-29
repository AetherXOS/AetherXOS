use crate::kernel_runtime::KernelRuntime;
use hypercore::hal::HAL;
use hypercore::interfaces::HardwareAbstraction;

#[cfg(target_arch = "x86_64")]
use hypercore::hal::x86_64::idt;

impl KernelRuntime {
    pub(super) fn finalize_runtime_interrupt_routing(self) {
        #[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
        idt::init_dispatcher(self.dispatcher);
        #[cfg(all(not(feature = "dispatcher"), target_arch = "x86_64"))]
        idt::init_dispatcher(());
    }
}

pub(super) fn enable_runtime_interrupts() {
    HAL::enable_interrupts();
}
