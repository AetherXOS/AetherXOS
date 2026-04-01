use super::*;

#[inline(always)]
#[cfg(feature = "vfs")]
pub(super) fn write_mount_record_words(
    out: &mut [usize],
    base: usize,
    record: &crate::kernel::vfs_control::MountRecord,
) {
    out[base] = record.id;
    out[base + 1] = record.fs_kind;
    out[base + 2] = record.path_len;
}

#[inline(always)]
pub(crate) fn parse_process_priority(priority: usize) -> Option<u8> {
    if priority > PROCESS_PRIORITY_MAX {
        None
    } else {
        Some(priority as u8)
    }
}

#[inline(always)]
pub(super) fn write_user_words<const N: usize>(ptr: usize, len: usize, words: [usize; N]) -> usize {
    let required = required_bytes(N);
    with_user_write_words(ptr, len, N, |out| {
        out.copy_from_slice(&words);
        required
    })
    .unwrap_or_else(|err| err)
}

#[inline(always)]
pub(crate) fn encode_core_pressure_class(
    class: crate::kernel::pressure::CorePressureClass,
) -> usize {
    match class {
        crate::kernel::pressure::CorePressureClass::Nominal => 0,
        crate::kernel::pressure::CorePressureClass::Elevated => 1,
        crate::kernel::pressure::CorePressureClass::High => 2,
        crate::kernel::pressure::CorePressureClass::Critical => 3,
    }
}

#[inline(always)]
pub(crate) fn encode_scheduler_pressure_class(
    class: crate::kernel::pressure::SchedulerPressureClass,
) -> usize {
    match class {
        crate::kernel::pressure::SchedulerPressureClass::Nominal => 0,
        crate::kernel::pressure::SchedulerPressureClass::Elevated => 1,
        crate::kernel::pressure::SchedulerPressureClass::High => 2,
        crate::kernel::pressure::SchedulerPressureClass::Critical => 3,
    }
}

#[inline(always)]
pub(crate) fn write_core_pressure_snapshot_words(
    out: &mut [usize],
    pressure: crate::kernel::pressure::CorePressureSnapshot,
) {
    out[0] = pressure.schema_version as usize;
    out[1] = pressure.online_cpus;
    out[2] = pressure.runqueue_total;
    out[3] = pressure.runqueue_max;
    out[4] = pressure.runqueue_avg_milli;
    out[5] = pressure.rt_starvation_alert as usize;
    out[6] = pressure.rt_forced_reschedules as usize;
    out[7] = pressure.watchdog_stall_detections as usize;
    out[8] = pressure.net_queue_limit;
    out[9] = pressure.net_rx_depth;
    out[10] = pressure.net_tx_depth;
    out[11] = pressure.net_saturation_percent;
    out[12] = pressure.lb_imbalance_p50;
    out[13] = pressure.lb_imbalance_p90;
    out[14] = pressure.lb_imbalance_p99;
    out[15] = pressure.lb_prefer_local_forced_moves as usize;
    out[16] = encode_core_pressure_class(pressure.class);
    out[17] = encode_scheduler_pressure_class(pressure.scheduler_class);
}

#[inline(always)]
pub(super) fn write_launch_context_response(
    ptr: usize,
    len: usize,
    process_id: crate::interfaces::task::ProcessId,
    task_id: crate::interfaces::task::TaskId,
    entry: usize,
    image_pages: usize,
    image_segments: usize,
    mapped_regions: usize,
    mapped_pages: usize,
    cr3: usize,
    return_value: usize,
) -> usize {
    with_user_write_words(ptr, len, PROCESS_LAUNCH_CTX_WORDS, |out| {
        write_launch_context_words(
            out,
            process_id.0,
            task_id.0,
            entry,
            image_pages,
            image_segments,
            mapped_regions,
            mapped_pages,
            cr3,
        );
        return_value
    })
    .unwrap_or_else(|err| err)
}

#[inline(always)]
pub(crate) fn upcall_entry_pc_valid(entry_pc: usize) -> bool {
    entry_pc >= USER_SPACE_BOTTOM_INCLUSIVE && entry_pc < USER_SPACE_TOP_EXCLUSIVE
}

#[inline(always)]
pub(crate) fn current_process_id() -> Option<usize> {
    #[cfg(feature = "process_abstraction")]
    {
        let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
        let current_tid = crate::interfaces::task::TaskId(
            cpu.current_task.load(core::sync::atomic::Ordering::Relaxed),
        );
        crate::kernel::launch::process_id_by_task(current_tid).map(|p| p.0)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        None
    }
}

#[derive(Clone, Copy)]
pub(crate) enum BinarySwitch {
    Disabled,
    Enabled,
}

impl BinarySwitch {
    #[inline(always)]
    pub(crate) fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::Enabled),
            _ => None,
        }
    }

    #[inline(always)]
    pub(crate) fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum PowerOverrideMode {
    HighPerf,
    Balanced,
    PowerSave,
}

impl PowerOverrideMode {
    #[inline(always)]
    pub(crate) fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Self::HighPerf),
            1 => Some(Self::Balanced),
            2 => Some(Self::PowerSave),
            _ => None,
        }
    }

    #[inline(always)]
    pub(super) fn to_kernel(self) -> crate::kernel::power::PState {
        match self {
            Self::HighPerf => crate::kernel::power::PState::HighPerf,
            Self::Balanced => crate::kernel::power::PState::Balanced,
            Self::PowerSave => crate::kernel::power::PState::PowerSave,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CStateOverrideMode {
    C1,
    C2,
    C3,
}

impl CStateOverrideMode {
    #[inline(always)]
    pub(crate) fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Self::C1),
            1 => Some(Self::C2),
            2 => Some(Self::C3),
            _ => None,
        }
    }

    #[inline(always)]
    pub(super) fn to_kernel(self) -> crate::kernel::power::CState {
        match self {
            Self::C1 => crate::kernel::power::CState::C1,
            Self::C2 => crate::kernel::power::CState::C2,
            Self::C3 => crate::kernel::power::CState::C3,
        }
    }
}

#[cfg(feature = "networking")]
#[derive(Clone, Copy)]
pub(super) enum BackpressurePolicyMode {
    Drop,
    Defer,
    ForcePoll,
}

#[cfg(feature = "networking")]
impl BackpressurePolicyMode {
    #[inline(always)]
    pub(super) fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Self::Drop),
            1 => Some(Self::Defer),
            2 => Some(Self::ForcePoll),
            _ => None,
        }
    }

    #[inline(always)]
    pub(super) fn to_network(self) -> crate::modules::network::bridge::BackpressurePolicy {
        match self {
            Self::Drop => crate::modules::network::bridge::BackpressurePolicy::Drop,
            Self::Defer => crate::modules::network::bridge::BackpressurePolicy::Defer,
            Self::ForcePoll => crate::modules::network::bridge::BackpressurePolicy::ForcePoll,
        }
    }
}
