use crate::kernel_runtime::interrupts;
use crate::kernel_runtime::KernelRuntime;

#[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
use hypercore::interfaces::Dispatcher;

impl KernelRuntime {
    pub(super) fn register_runtime_irq_handlers(&self) {
        #[cfg(all(feature = "dispatcher", target_arch = "x86_64"))]
        {
            let irq_base = hypercore::config::KernelConfig::irq_vector_base();
            self.dispatcher
                .register_handler(irq_base, interrupts::timer_tick_handler);

            hypercore::hal::x86_64::input::init();
            self.dispatcher.register_handler(
                irq_base + 1,
                hypercore::hal::x86_64::input::handle_keyboard_irq,
            );
            self.dispatcher.register_handler(
                irq_base + 12,
                hypercore::hal::x86_64::input::handle_mouse_irq,
            );
        }
    }
}
