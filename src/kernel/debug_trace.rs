use crate::generated_consts::CORE_CRASH_LOG_CAPACITY;
use alloc::{vec, vec::Vec};
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

const TRACE_TEXT_LIMIT: usize = 24;
const TRACE_FLAG_HAS_VALUE: u8 = 1;
const TRACE_FLAG_IS_DUMP: u8 = 2;
const TRACE_FLAG_WARN: u8 = 4;
const TRACE_FLAG_FAULT: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TraceCategory {
    Core = 0,
    Launch = 1,
    Loader = 2,
    Task = 3,
    Memory = 4,
    Scheduler = 5,
    Fault = 6,
}

crate::impl_enum_u8_option_conversions!(TraceCategory {
    Core,
    Launch,
    Loader,
    Task,
    Memory,
    Scheduler,
    Fault,
});

crate::impl_enum_str_conversions!(TraceCategory {
    Core => "core",
    Launch => "launch",
    Loader => "loader",
    Task => "task",
    Memory => "memory",
    Scheduler => "scheduler",
    Fault => "fault",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceSeverity {
    Trace,
    Warn,
    Fault,
}

#[derive(Debug, Clone, Copy)]
pub struct TraceRecord {
    pub seq: u64,
    pub flags: u8,
    pub category: u8,
    pub scope_len: u8,
    pub stage_len: u8,
    pub value: u64,
    pub scope: [u8; TRACE_TEXT_LIMIT],
    pub stage: [u8; TRACE_TEXT_LIMIT],
}

impl TraceRecord {
    pub const EMPTY: Self = Self {
        seq: 0,
        flags: 0,
        category: TraceCategory::Core as u8,
        scope_len: 0,
        stage_len: 0,
        value: 0,
        scope: [0; TRACE_TEXT_LIMIT],
        stage: [0; TRACE_TEXT_LIMIT],
    };

    pub fn scope_str(&self) -> &str {
        core::str::from_utf8(&self.scope[..self.scope_len as usize]).unwrap_or("?")
    }

    pub fn stage_str(&self) -> &str {
        core::str::from_utf8(&self.stage[..self.stage_len as usize]).unwrap_or("?")
    }

    pub fn category_str(&self) -> &'static str {
        TraceCategory::from_u8(self.category)
            .map(|category| category.as_str())
            .unwrap_or("unknown")
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TraceStats {
    pub events: u64,
    pub valued_events: u64,
    pub dump_events: u64,
    pub warn_events: u64,
    pub fault_events: u64,
    pub context_events: u64,
    pub dropped_history: u64,
    pub latest_seq: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TraceCategoryStats {
    pub core: u64,
    pub launch: u64,
    pub loader: u64,
    pub task: u64,
    pub memory: u64,
    pub scheduler: u64,
    pub fault: u64,
    pub unknown: u64,
}

static TRACE_LOG: Mutex<[TraceRecord; CORE_CRASH_LOG_CAPACITY]> =
    Mutex::new([TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY]);
static TRACE_NEXT_SEQ: AtomicU64 = AtomicU64::new(0);
static TRACE_LOCK_DROPS: AtomicU64 = AtomicU64::new(0);

fn copy_trace_text(dst: &mut [u8; TRACE_TEXT_LIMIT], src: &str) -> u8 {
    let bytes = src.as_bytes();
    let len = core::cmp::min(bytes.len(), TRACE_TEXT_LIMIT);
    dst[..len].copy_from_slice(&bytes[..len]);
    len as u8
}

pub fn record(scope: &str, stage: &str, value: Option<u64>, is_dump: bool) {
    record_with_metadata(
        scope,
        stage,
        value,
        is_dump,
        TraceSeverity::Trace,
        TraceCategory::Core,
    );
}

pub fn record_with_severity(
    scope: &str,
    stage: &str,
    value: Option<u64>,
    is_dump: bool,
    severity: TraceSeverity,
) {
    record_with_metadata(scope, stage, value, is_dump, severity, TraceCategory::Core);
}

pub fn record_with_metadata(
    scope: &str,
    stage: &str,
    value: Option<u64>,
    is_dump: bool,
    severity: TraceSeverity,
    category: TraceCategory,
) {
    let seq = TRACE_NEXT_SEQ.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    let idx = (seq as usize) % CORE_CRASH_LOG_CAPACITY;
    let mut record = TraceRecord::EMPTY;
    record.seq = seq;
    record.category = category as u8;
    record.scope_len = copy_trace_text(&mut record.scope, scope);
    record.stage_len = copy_trace_text(&mut record.stage, stage);
    if let Some(v) = value {
        record.flags |= TRACE_FLAG_HAS_VALUE;
        record.value = v;
    }
    if is_dump {
        record.flags |= TRACE_FLAG_IS_DUMP;
    }
    match severity {
        TraceSeverity::Trace => {}
        TraceSeverity::Warn => record.flags |= TRACE_FLAG_WARN,
        TraceSeverity::Fault => record.flags |= TRACE_FLAG_FAULT,
    }
    if let Some(mut records) = TRACE_LOG.try_lock() {
        records[idx] = record;
    } else {
        TRACE_LOCK_DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

#[inline(always)]
pub fn record_optional(scope: &str, stage: &str, value: Option<u64>, is_dump: bool) {
    if crate::config::KernelConfig::debug_trace_enabled() {
        record(scope, stage, value, is_dump);
    }
}

pub fn record_register_snapshot(scope: &str, pc: u64, sp: u64, aux0: u64, aux1: u64) {
    record_optional(scope, "pc", Some(pc), false);
    record_optional(scope, "sp", Some(sp), false);
    record_optional(scope, "aux0", Some(aux0), false);
    record_optional(scope, "aux1", Some(aux1), false);
}

pub fn record_warn(scope: &str, stage: &str, value: Option<u64>) {
    if crate::config::KernelConfig::debug_trace_enabled() {
        record_with_metadata(
            scope,
            stage,
            value,
            false,
            TraceSeverity::Warn,
            TraceCategory::Fault,
        );
    }
}

pub fn record_fault(scope: &str, stage: &str, value: Option<u64>) {
    if crate::config::KernelConfig::debug_trace_enabled() {
        record_with_metadata(
            scope,
            stage,
            value,
            false,
            TraceSeverity::Fault,
            TraceCategory::Fault,
        );
    }
}

pub fn record_bytes_preview(scope: &str, stage: &str, bytes: &[u8]) {
    let mut folded = 0u64;
    for (idx, byte) in bytes.iter().copied().take(8).enumerate() {
        folded |= (byte as u64) << (idx * 8);
    }
    record_optional(scope, stage, Some(folded), true);
    record_optional(scope, "len", Some(bytes.len() as u64), false);
}

pub fn record_kernel_context(scope: &str, stage: &str, value: Option<u64>) {
    if crate::config::KernelConfig::debug_trace_enabled() {
        record_with_metadata(
            scope,
            stage,
            value,
            false,
            TraceSeverity::Trace,
            TraceCategory::Core,
        );
    }
    #[cfg(target_os = "none")]
    {
        let cpu_id = crate::hal::cpu::id();
        record_optional(scope, "cpu", Some(cpu_id as u64), false);
        if let Some(cpu) = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() } {
            let tid = cpu.current_task.load(Ordering::Relaxed);
            record_optional(scope, "task", Some(tid as u64), false);
        }
    }
}

pub fn recent_into(out: &mut [TraceRecord]) -> usize {
    if out.is_empty() {
        return 0;
    }

    let total = core::cmp::min(
        TRACE_NEXT_SEQ.load(Ordering::Relaxed) as usize,
        CORE_CRASH_LOG_CAPACITY,
    );
    if total == 0 {
        return 0;
    }

    let records = TRACE_LOG.lock();
    let n = core::cmp::min(out.len(), total);
    let oldest = if total == CORE_CRASH_LOG_CAPACITY {
        (TRACE_NEXT_SEQ.load(Ordering::Relaxed) as usize) % CORE_CRASH_LOG_CAPACITY
    } else {
        0
    };
    let start = total.saturating_sub(n);
    let mut cursor = (oldest + start) % CORE_CRASH_LOG_CAPACITY;
    let mut written = 0usize;

    while written < n {
        let record = records[cursor];
        if record.seq != 0 {
            out[written] = record;
            written += 1;
        }
        cursor = (cursor + 1) % CORE_CRASH_LOG_CAPACITY;
    }

    written
}

pub fn recent_records_vec_copy() -> Vec<TraceRecord> {
    let mut out = vec![TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_into(&mut out);
    out.truncate(written);
    out
}

pub fn recent_records_for_category(category: TraceCategory, out: &mut [TraceRecord]) -> usize {
    if out.is_empty() {
        return 0;
    }
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_into(&mut recent);
    let mut matched = 0usize;
    for record in recent.iter().take(written) {
        if record.category == category as u8 && matched < out.len() {
            out[matched] = *record;
            matched += 1;
        }
    }
    matched
}

pub fn latest_record() -> Option<TraceRecord> {
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_into(&mut recent);
    recent.into_iter().take(written).last().filter(|record| record.seq != 0)
}

pub fn latest_record_for_category(category: TraceCategory) -> Option<TraceRecord> {
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_into(&mut recent);
    recent
        .into_iter()
        .take(written)
        .rev()
        .find(|record| record.seq != 0 && record.category == category as u8)
}

#[inline(always)]
pub fn event_count() -> u64 {
    TRACE_NEXT_SEQ.load(Ordering::Relaxed)
}

pub fn stats() -> TraceStats {
    let events = TRACE_NEXT_SEQ.load(Ordering::Relaxed);
    let dropped_overflow = events.saturating_sub(CORE_CRASH_LOG_CAPACITY as u64);
    let dropped_lock = TRACE_LOCK_DROPS.load(Ordering::Relaxed);
    let mut valued_events = 0u64;
    let mut dump_events = 0u64;
    let mut warn_events = 0u64;
    let mut fault_events = 0u64;
    let mut context_events = 0u64;
    let mut latest_seq = 0u64;
    let records = TRACE_LOG.lock();
    for record in records.iter() {
        if record.seq == 0 {
            continue;
        }
        latest_seq = latest_seq.max(record.seq);
        if (record.flags & TRACE_FLAG_HAS_VALUE) != 0 {
            valued_events = valued_events.saturating_add(1);
        }
        if (record.flags & TRACE_FLAG_IS_DUMP) != 0 {
            dump_events = dump_events.saturating_add(1);
        }
        if (record.flags & TRACE_FLAG_WARN) != 0 {
            warn_events = warn_events.saturating_add(1);
        }
        if (record.flags & TRACE_FLAG_FAULT) != 0 {
            fault_events = fault_events.saturating_add(1);
        }
        if record.stage_str() == "cpu" || record.stage_str() == "task" {
            context_events = context_events.saturating_add(1);
        }
    }

    TraceStats {
        events,
        valued_events,
        dump_events,
        warn_events,
        fault_events,
        context_events,
        dropped_history: dropped_overflow.saturating_add(dropped_lock),
        latest_seq,
    }
}

pub fn category_stats() -> TraceCategoryStats {
    let mut out = TraceCategoryStats::default();
    let records = TRACE_LOG.lock();
    for record in records.iter() {
        if record.seq == 0 {
            continue;
        }
        match record.category {
            x if x == TraceCategory::Core as u8 => out.core = out.core.saturating_add(1),
            x if x == TraceCategory::Launch as u8 => out.launch = out.launch.saturating_add(1),
            x if x == TraceCategory::Loader as u8 => out.loader = out.loader.saturating_add(1),
            x if x == TraceCategory::Task as u8 => out.task = out.task.saturating_add(1),
            x if x == TraceCategory::Memory as u8 => out.memory = out.memory.saturating_add(1),
            x if x == TraceCategory::Scheduler as u8 => {
                out.scheduler = out.scheduler.saturating_add(1)
            }
            x if x == TraceCategory::Fault as u8 => out.fault = out.fault.saturating_add(1),
            _ => out.unknown = out.unknown.saturating_add(1),
        }
    }
    out
}

pub fn dump_to_klog() {
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_into(&mut recent);
    crate::klog_error!("[KERNEL DUMP] trace_events={}", written);
    for record in recent.iter().take(written) {
        let severity = if (record.flags & TRACE_FLAG_FAULT) != 0 {
            "fault"
        } else if (record.flags & TRACE_FLAG_WARN) != 0 {
            "warn"
        } else {
            "trace"
        };
        if (record.flags & TRACE_FLAG_HAS_VALUE) != 0 {
            crate::klog_error!(
                "[KERNEL DUMP] trace seq={} sev={} cat={} {} {} value={:#x}",
                record.seq,
                severity,
                record.category_str(),
                record.scope_str(),
                record.stage_str(),
                record.value
            );
        } else {
            crate::klog_error!(
                "[KERNEL DUMP] trace seq={} sev={} cat={} {} {}",
                record.seq,
                severity,
                record.category_str(),
                record.scope_str(),
                record.stage_str()
            );
        }
    }
}

pub fn dump_category_to_klog(category: TraceCategory, limit: usize) {
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_records_for_category(category, &mut recent);
    let start = written.saturating_sub(limit);
    crate::klog_error!(
        "[KERNEL DUMP] trace_tail cat={} count={}",
        category.as_str(),
        written.saturating_sub(start)
    );
    for record in recent.iter().take(written).skip(start) {
        let severity = if (record.flags & TRACE_FLAG_FAULT) != 0 {
            "fault"
        } else if (record.flags & TRACE_FLAG_WARN) != 0 {
            "warn"
        } else {
            "trace"
        };
        if (record.flags & TRACE_FLAG_HAS_VALUE) != 0 {
            crate::klog_error!(
                "[KERNEL DUMP] trace_tail seq={} sev={} {} {} value={:#x}",
                record.seq,
                severity,
                record.scope_str(),
                record.stage_str(),
                record.value
            );
        } else {
            crate::klog_error!(
                "[KERNEL DUMP] trace_tail seq={} sev={} {} {}",
                record.seq,
                severity,
                record.scope_str(),
                record.stage_str()
            );
        }
    }
}

pub fn dump_to_early_serial() {
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let written = recent_into(&mut recent);
    crate::hal::serial::write_raw("[EARLY SERIAL] trace dump begin\n");
    for record in recent.iter().take(written) {
        if (record.flags & TRACE_FLAG_HAS_VALUE) != 0 {
            crate::hal::serial::write_trace_hex(record.scope_str(), record.stage_str(), record.value);
        } else {
            crate::hal::serial::write_trace(record.scope_str(), record.stage_str());
        }
    }
    crate::hal::serial::write_raw("[EARLY SERIAL] trace dump end\n");
}

pub fn recent_records_copy() -> [TraceRecord; CORE_CRASH_LOG_CAPACITY] {
    let mut recent = [TraceRecord::EMPTY; CORE_CRASH_LOG_CAPACITY];
    let _ = recent_into(&mut recent);
    recent
}

#[cfg(test)]
#[path = "debug_trace/tests.rs"]
mod tests;

