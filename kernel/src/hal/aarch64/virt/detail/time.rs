use crate::hal::common::virt::{VirtStatus, TIME_BASIC, TIME_CNTV_READY, TIME_NONE};

pub(super) fn time_detail(status: VirtStatus, timer_frequency_hz: u64) -> &'static str {
    if status.vm_launch_ready && timer_frequency_hz != 0 {
        TIME_CNTV_READY
    } else if status.caps.hypervisor_present {
        TIME_BASIC
    } else {
        TIME_NONE
    }
}
