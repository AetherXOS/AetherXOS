use crate::hal::common::virt::{
    current_virtualization_runtime_governor, GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_RELAXED,
};
use crate::interfaces::Scheduler;

pub const CORE_PRESSURE_SCHEMA_VERSION: u16 = 2;

const RUNQUEUE_ELEVATED_THRESHOLD: usize = 8;
const RUNQUEUE_HIGH_THRESHOLD: usize = 16;
const RUNQUEUE_CRITICAL_THRESHOLD: usize = 32;
const SATURATION_ELEVATED_PERCENT: usize = 50;
const SATURATION_HIGH_PERCENT: usize = 80;
const SATURATION_CRITICAL_PERCENT: usize = 95;
const RUNQUEUE_AVG_MILLI_SCALE: usize = 1000;
const PERCENT_SCALE: usize = 100;
const IMBALANCE_ELEVATED_THRESHOLD: usize = 4;
const IMBALANCE_HIGH_THRESHOLD: usize = 8;
const IMBALANCE_CRITICAL_THRESHOLD: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorePressureClass {
    Nominal,
    Elevated,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerPressureClass {
    Nominal,
    Elevated,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy)]
pub struct CorePressureSnapshot {
    pub schema_version: u16,
    pub online_cpus: usize,
    pub runqueue_total: usize,
    pub runqueue_max: usize,
    pub runqueue_avg_milli: usize,
    pub rt_starvation_alert: bool,
    pub rt_forced_reschedules: u64,
    pub watchdog_stall_detections: u64,
    pub net_queue_limit: usize,
    pub net_rx_depth: usize,
    pub net_tx_depth: usize,
    pub net_saturation_percent: usize,
    pub lb_imbalance_p50: usize,
    pub lb_imbalance_p90: usize,
    pub lb_imbalance_p99: usize,
    pub lb_prefer_local_forced_moves: u64,
    pub class: CorePressureClass,
    pub scheduler_class: SchedulerPressureClass,
}

fn classify_pressure(
    runqueue_max: usize,
    saturation_percent: usize,
    rt_starvation_alert: bool,
    watchdog_stall_detections: u64,
    latency_bias: &'static str,
) -> CorePressureClass {
    let critical_runqueue = governor_adjusted_threshold(RUNQUEUE_CRITICAL_THRESHOLD, latency_bias);
    let high_runqueue = governor_adjusted_threshold(RUNQUEUE_HIGH_THRESHOLD, latency_bias);
    let elevated_runqueue = governor_adjusted_threshold(RUNQUEUE_ELEVATED_THRESHOLD, latency_bias);
    let critical_saturation =
        governor_adjusted_threshold(SATURATION_CRITICAL_PERCENT, latency_bias);
    let high_saturation = governor_adjusted_threshold(SATURATION_HIGH_PERCENT, latency_bias);
    let elevated_saturation =
        governor_adjusted_threshold(SATURATION_ELEVATED_PERCENT, latency_bias);

    if watchdog_stall_detections > 0
        || runqueue_max >= critical_runqueue
        || saturation_percent >= critical_saturation
    {
        return CorePressureClass::Critical;
    }

    if rt_starvation_alert || runqueue_max >= high_runqueue || saturation_percent >= high_saturation
    {
        return CorePressureClass::High;
    }

    if runqueue_max >= elevated_runqueue || saturation_percent >= elevated_saturation {
        return CorePressureClass::Elevated;
    }

    CorePressureClass::Nominal
}

fn classify_scheduler_pressure(
    runqueue_max: usize,
    lb_imbalance_p99: usize,
    prefer_local_forced_moves: u64,
    rt_starvation_alert: bool,
    latency_bias: &'static str,
) -> SchedulerPressureClass {
    let critical_runqueue = governor_adjusted_threshold(RUNQUEUE_CRITICAL_THRESHOLD, latency_bias);
    let high_runqueue = governor_adjusted_threshold(RUNQUEUE_HIGH_THRESHOLD, latency_bias);
    let elevated_runqueue = governor_adjusted_threshold(RUNQUEUE_ELEVATED_THRESHOLD, latency_bias);
    let critical_imbalance =
        governor_adjusted_threshold(IMBALANCE_CRITICAL_THRESHOLD, latency_bias);
    let high_imbalance = governor_adjusted_threshold(IMBALANCE_HIGH_THRESHOLD, latency_bias);
    let elevated_imbalance =
        governor_adjusted_threshold(IMBALANCE_ELEVATED_THRESHOLD, latency_bias);

    if rt_starvation_alert
        || prefer_local_forced_moves > 0
        || runqueue_max >= critical_runqueue
        || lb_imbalance_p99 >= critical_imbalance
    {
        return SchedulerPressureClass::Critical;
    }

    if runqueue_max >= high_runqueue || lb_imbalance_p99 >= high_imbalance {
        return SchedulerPressureClass::High;
    }

    if runqueue_max >= elevated_runqueue || lb_imbalance_p99 >= elevated_imbalance {
        return SchedulerPressureClass::Elevated;
    }

    SchedulerPressureClass::Nominal
}

#[inline(always)]
fn governor_adjusted_threshold(base: usize, latency_bias: &'static str) -> usize {
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => base.saturating_sub((base / 4).max(1)).max(1),
        GOVERNOR_BIAS_RELAXED => base.saturating_add((base / 4).max(1)),
        _ => base.max(1),
    }
}

pub fn snapshot() -> CorePressureSnapshot {
    let (online_cpus, runqueue_total, runqueue_max) = {
        let cpus = crate::hal::smp::CPUS.lock();
        let mut total = 0usize;
        let mut max = 0usize;
        for cpu in cpus.iter() {
            let len = cpu.scheduler.lock().runqueue_len();
            total = total.saturating_add(len);
            max = core::cmp::max(max, len);
        }
        (cpus.len(), total, max)
    };

    let runqueue_avg_milli = if online_cpus == 0 {
        0
    } else {
        runqueue_total.saturating_mul(RUNQUEUE_AVG_MILLI_SCALE) / online_cpus
    };

    let rt = crate::kernel::rt_preemption::stats();
    let wd = crate::kernel::watchdog::stats();
    let net = crate::kernel::net_core::stats();
    let lb = crate::kernel::load_balance::stats_snapshot();
    let governor = current_virtualization_runtime_governor();

    let queue_limit = core::cmp::max(net.queue_limit, 1);
    let rx_percent = (net.rx_depth.saturating_mul(PERCENT_SCALE)) / queue_limit;
    let tx_percent = (net.tx_depth.saturating_mul(PERCENT_SCALE)) / queue_limit;
    let saturation_percent = core::cmp::max(rx_percent, tx_percent);

    let class = classify_pressure(
        runqueue_max,
        saturation_percent,
        rt.starvation_alert,
        wd.stall_detections,
        governor.latency_bias,
    );
    let scheduler_class = classify_scheduler_pressure(
        runqueue_max,
        lb.imbalance_p99,
        lb.prefer_local_forced_moves,
        rt.starvation_alert,
        governor.latency_bias,
    );

    CorePressureSnapshot {
        schema_version: CORE_PRESSURE_SCHEMA_VERSION,
        online_cpus,
        runqueue_total,
        runqueue_max,
        runqueue_avg_milli,
        rt_starvation_alert: rt.starvation_alert,
        rt_forced_reschedules: rt.forced_reschedules,
        watchdog_stall_detections: wd.stall_detections,
        net_queue_limit: net.queue_limit,
        net_rx_depth: net.rx_depth,
        net_tx_depth: net.tx_depth,
        net_saturation_percent: saturation_percent,
        lb_imbalance_p50: lb.imbalance_p50,
        lb_imbalance_p90: lb.imbalance_p90,
        lb_imbalance_p99: lb.imbalance_p99,
        lb_prefer_local_forced_moves: lb.prefer_local_forced_moves,
        class,
        scheduler_class,
    }
}

/// Called by the memory-pressure kernel service daemon on each loop iteration.
/// Takes a snapshot and logs a warning if the system is under high or critical
/// pressure.  Full OOM killing is a future extension.
pub fn on_pressure_tick() {
    let s = snapshot();
    if matches!(
        s.class,
        CorePressureClass::Critical | CorePressureClass::High
    ) {
        crate::klog_warn!(
            "memory pressure class={:?} runqueue_max={} saturation={}%",
            s.class,
            s.runqueue_max,
            s.net_saturation_percent,
        );
    }
}

#[cfg(test)]
#[path = "pressure/tests.rs"]
mod tests;
