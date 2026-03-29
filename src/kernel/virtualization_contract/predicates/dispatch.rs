use crate::hal::common::virt::{runtime_dispatch_window, runtime_scheduler_lane};

#[inline(always)]
pub fn virtualization_dispatch_contract_holds(
    dispatch_class: &'static str,
    scheduler_lane: &'static str,
    preemption_policy: &'static str,
    dispatch_window: &'static str,
) -> bool {
    scheduler_lane == runtime_scheduler_lane(dispatch_class)
        && dispatch_window == runtime_dispatch_window(dispatch_class, preemption_policy)
}
