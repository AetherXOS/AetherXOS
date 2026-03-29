mod arch;
mod schedule;
mod types;

use hypercore::hal::HAL;

pub(crate) fn timer_tick_handler(_irq: u8) {
    use core::sync::atomic::Ordering;
    use hypercore::interfaces::task::TaskId;

    hypercore::kernel::debug_trace::record_optional("timer.tick", "handler_entered", None, false);

    hypercore::kernel::load_balance::maybe_periodic_rebalance();

    let cpu: &'static hypercore::kernel::cpu_local::CpuLocal =
        unsafe { hypercore::kernel::cpu_local::CpuLocal::get() };
    hypercore::kernel::watchdog::on_timer_tick(cpu);

    let current_tid = TaskId(cpu.current_task.load(Ordering::Relaxed));
    let switch_info = schedule::prepare_scheduler_switch(cpu, current_tid);

    if let Some(switch_info) = switch_info {
        #[cfg(target_arch = "x86_64")]
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] timer switch prepared\n");
        arch::apply_arch_switch_state(cpu, &switch_info);
        unsafe {
            hypercore::kernel::rt_preemption::on_context_switch();
            HAL::context_switch(switch_info.current_sp_ptr, switch_info.next_sp);
        }
    }
}
