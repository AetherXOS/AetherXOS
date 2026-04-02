mod arch;
mod schedule;
mod types;

use aethercore::hal::HAL;

pub(crate) fn timer_tick_handler(_irq: u8) {
    use core::sync::atomic::Ordering;
    use aethercore::interfaces::task::TaskId;

    aethercore::kernel::debug_trace::record_optional("timer.tick", "handler_entered", None, false);

    aethercore::kernel::load_balance::maybe_periodic_rebalance();

    let cpu: &'static aethercore::kernel::cpu_local::CpuLocal =
        unsafe { aethercore::kernel::cpu_local::CpuLocal::get() };
    aethercore::kernel::watchdog::on_timer_tick(cpu);

    let current_tid = TaskId(cpu.current_task.load(Ordering::Relaxed));
    let switch_info = schedule::prepare_scheduler_switch(cpu, current_tid);

    if let Some(switch_info) = switch_info {
        #[cfg(target_arch = "x86_64")]
        aethercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] timer switch prepared\n");
        arch::apply_arch_switch_state(cpu, &switch_info);
        unsafe {
            aethercore::kernel::rt_preemption::on_context_switch();
            HAL::context_switch(switch_info.current_sp_ptr, switch_info.next_sp);
        }
    }
}
