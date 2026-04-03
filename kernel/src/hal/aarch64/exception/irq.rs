use super::*;
#[path = "irq_storm.rs"]
mod irq_storm;
use irq_storm::IrqStormState;
pub(super) use irq_storm::{hottest_irq_snapshot, IrqRateTracker};

use crate::generated_consts::{
    AARCH64_TIMER_JITTER_TOLERANCE_TICKS,
};
use crate::hal::common::irq_catalog::{
    classify_irq_line, IrqLineDescriptor, IrqLineKind,
};
use crate::hal::common::irq::abs_diff_u64;
use crate::hal::aarch64::platform::irq::{
    IRQ_LINE_DESCRIPTORS,
};

const GIC_IAR_IRQ_ID_MASK: u32 = 0x3FF;
const GIC_SPURIOUS_IRQ_BASE: u32 = 1020;
const TIMER_REARM_HZ_DIVISOR: u64 = 1000;
const UART_RX_BURST_BYTES: usize = 32;

#[inline(always)]
fn irq_line_descriptor(irq_id: u32) -> IrqLineDescriptor<u32> {
    classify_irq_line(irq_id, &IRQ_LINE_DESCRIPTORS, "generic")
}

fn handle_timer_irq(now_counter: u64) {
    TIMER_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
    let last = TIMER_LAST_IRQ_COUNTER.swap(now_counter, Ordering::Relaxed);
    if last != 0 {
        let delta = now_counter.wrapping_sub(last);
        let target = crate::hal::aarch64::timer::GenericTimer::last_programmed_ticks().max(1);
        let jitter = abs_diff_u64(delta, target);
        if jitter > AARCH64_TIMER_JITTER_TOLERANCE_TICKS {
            TIMER_IRQ_JITTER_EVENTS.fetch_add(1, Ordering::Relaxed);
        }
    }
    crate::hal::aarch64::timer::GenericTimer::set_timer(
        crate::hal::aarch64::timer::GenericTimer::frequency() / TIMER_REARM_HZ_DIVISOR,
    );
}

fn handle_uart_irq() {
    let serial = crate::hal::aarch64::serial::SERIAL1.lock();
    let mut rx_buf = [0u8; UART_RX_BURST_BYTES];
    let n = serial.recv_burst(&mut rx_buf);
    if n > 0 {
        if let Some(tty) = crate::kernel::tty::GLOBAL_TTY_REGISTRY
            .lock()
            .get(crate::kernel::tty::TtyId::new(0))
        {
            tty.push_input(&rx_buf[..n]);
        }
    }
    serial.clear_interrupts();
}

#[inline(always)]
fn handle_irq_line(irq_id: u32, now_counter: u64) {
    let descriptor = irq_line_descriptor(irq_id);
    let storm = IrqStormState::new(now_counter);
    let (in_window, global_decision) = storm.record_global();
    if global_decision.should_log {
        crate::klog_debug!(
            "AArch64 IRQ {} kind={} window_events={} storm={}",
            irq_id,
            descriptor.label,
            in_window,
            global_decision.in_storm
        );
    }

    if let Some((line_count, line_decision)) = storm.record_per_line(irq_id) {
        if line_decision.should_log {
            crate::klog_warn!(
                "AArch64 IRQ line storm irq={} kind={} line_window_events={} threshold={}",
                irq_id,
                descriptor.label,
                line_count,
                storm.per_line_threshold()
            );
        }
    }

    match descriptor.kind {
        IrqLineKind::Timer => handle_timer_irq(now_counter),
        IrqLineKind::Serial => handle_uart_irq(),
        IrqLineKind::Generic => {}
        IrqLineKind::TlbShootdown => {}
    }
}

fn handle_genuine_irq(gic: &mut crate::hal::aarch64::gic::Gic, irq_id: u32) {
    let now_counter = crate::hal::aarch64::timer::GenericTimer::counter();
    handle_irq_line(irq_id, now_counter);

    use crate::interfaces::InterruptController;
    unsafe { gic.end_of_interrupt(irq_id) };
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_irq(_frame: &mut ExceptionFrame) {
    IRQ_TOTAL_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);

    let mut gic = crate::hal::aarch64::gic::GIC.lock();
    let iar = gic.read_iar();
    let irq_id = iar & GIC_IAR_IRQ_ID_MASK;

    if irq_id < GIC_SPURIOUS_IRQ_BASE {
        handle_genuine_irq(&mut gic, irq_id);
    } else {
        IRQ_SPURIOUS_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    }
}
