#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrqRoute<Id, Handler> {
    pub id: Id,
    pub handler: Handler,
}

use crate::interfaces::InterruptController;

impl<Id, Handler> IrqRoute<Id, Handler> {
    pub const fn new(id: Id, handler: Handler) -> Self {
        Self { id, handler }
    }
}

#[inline(always)]
pub fn register_irq_routes<Id: Copy, Handler: Copy>(
    routes: &[IrqRoute<Id, Handler>],
    mut register: impl FnMut(Id, Handler),
) {
    for route in routes {
        register(route.id, route.handler);
    }
}

#[inline(always)]
pub fn register_irq_ids<Id: Copy>(ids: &[Id], mut register: impl FnMut(Id)) {
    for id in ids {
        register(*id);
    }
}

#[inline(always)]
pub fn enable_interrupt_lines(controller: &mut impl InterruptController, ids: &[u32]) {
    register_irq_ids(ids, |irq| {
        // Safety: caller provides IRQ lines valid for the active interrupt controller.
        unsafe { controller.enable_interrupt(irq) }
    });
}
