#[cfg(feature = "dispatcher")]
use crate::modules::dispatcher::selector::ActiveDispatcher;
#[cfg(target_os = "none")]
use crate::kernel::syscalls::syscalls_consts::x86;
#[cfg(target_os = "none")]
use crate::hal::common::irq_catalog::IrqLineKind;

#[cfg(feature = "dispatcher")]
static mut DISPATCHER: Option<ActiveDispatcher> = None;

#[cfg(feature = "dispatcher")]
pub fn init_dispatcher(d: ActiveDispatcher) {
    unsafe {
        DISPATCHER = Some(d);
    }
}

#[cfg(not(feature = "dispatcher"))]
pub fn init_dispatcher(_d: ()) {}

#[cfg(target_os = "none")]
#[inline(always)]
pub(super) fn try_dispatch_vector(vector: u8) -> bool {
    #[cfg(feature = "dispatcher")]
    {
        use crate::interfaces::Dispatcher;
        if let Some(d) = unsafe { &*(&raw const DISPATCHER) } {
            d.dispatch(vector);
            return true;
        }
    }

    false
}

#[cfg(target_os = "none")]
#[inline(always)]
pub(super) fn dispatch_irq_vector(vector: u8) {
    let descriptor = super::metadata::irq_descriptor(vector);
    let allow_dispatch = crate::kernel::interrupt_guard::on_irq(vector);
    let is_timer = descriptor.kind == IrqLineKind::Timer;

    if crate::generated_consts::CORE_ENABLE_IRQ_TRACE && !is_timer {
        crate::klog_trace!("IRQ vector {} kind={} dispatched", vector, descriptor.label);
    }

    if allow_dispatch {
        let _ = try_dispatch_vector(vector);
    } else if crate::generated_consts::CORE_ENABLE_IRQ_TRACE {
        crate::klog_trace!(
            "IRQ vector {} kind={} dropped by storm protection",
            vector,
            descriptor.label
        );
    }

    unsafe {
        crate::hal::apic::eoi();
    }
}
