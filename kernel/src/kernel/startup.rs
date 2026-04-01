use crate::interfaces::task::{KernelTask, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use lazy_static::lazy_static;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StartupStage {
    BootStart,
    BootInfoCollected, // Limine protocol data parsed
    HeapInit,
    HalEarlyInit,
    PlatformServicesInit,
    IrqHandlersRegistered,
    PciEnumerated,
    IommuAttached,
    DriversInit,
    SmpInit,
    IdtReady,
    InterruptsEnabled,
    MainLoopEntered,
}

impl_enum_u8_default_conversions!(StartupStage {
    BootStart,
    BootInfoCollected,
    HeapInit,
    HalEarlyInit,
    PlatformServicesInit,
    IrqHandlersRegistered,
    PciEnumerated,
    IommuAttached,
    DriversInit,
    SmpInit,
    IdtReady,
    InterruptsEnabled,
    MainLoopEntered,
}, default = BootStart);

#[derive(Debug, Clone, Copy)]
pub struct StartupDiagnostics {
    pub transitions: u64,
    pub ordering_violations: u64,
    pub last_stage: StartupStage,
}

static STARTUP_TRANSITIONS: AtomicU64 = AtomicU64::new(0);
static STARTUP_ORDERING_VIOLATIONS: AtomicU64 = AtomicU64::new(0);
static STARTUP_LAST_STAGE: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
fn stage_to_u64(stage: StartupStage) -> u64 {
    stage.to_u8() as u64
}

#[inline(always)]
fn stage_from_u64(raw: u64) -> StartupStage {
    if raw > u64::from(u8::MAX) {
        return StartupStage::BootStart;
    }
    StartupStage::from_u8(raw as u8).expect("invalid startup stage")
}

pub fn mark_stage(stage: StartupStage) {
    let now = stage_to_u64(stage);
    let prev = STARTUP_LAST_STAGE.load(Ordering::Relaxed);
    if now < prev {
        STARTUP_ORDERING_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
    }
    STARTUP_LAST_STAGE.store(now, Ordering::Relaxed);
    STARTUP_TRANSITIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn diagnostics() -> StartupDiagnostics {
    StartupDiagnostics {
        transitions: STARTUP_TRANSITIONS.load(Ordering::Relaxed),
        ordering_violations: STARTUP_ORDERING_VIOLATIONS.load(Ordering::Relaxed),
        last_stage: stage_from_u64(STARTUP_LAST_STAGE.load(Ordering::Relaxed)),
    }
}

#[cfg(test)]
mod tests;

// ── Service manager ───────────────────────────────────────────────────────────

/// Well-known kernel internal services.  Each maps to a long-lived kernel task
/// created by `spawn_init_services()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InitService {
    /// Watchdog — detects hung tasks and fires NMI / panic.
    Watchdog = 0,
    /// Journal commit daemon — periodically flushes the VFS journal.
    JournalCommit = 1,
    /// Memory pressure handler — trims caches and invokes OOM killer.
    MemPressure = 2,
    /// Network RX/TX poll loop — processes queued packets.
    NetworkPoll = 3,
    /// Telemetry reporter — serialises kernel metrics to the diagnostics ring.
    Telemetry = 4,
}

const NUM_SERVICES: usize = 5;

/// State of a single kernel service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ServiceState {
    Stopped = 0,
    Starting = 1,
    Running = 2,
    Failed = 3,
}

impl_enum_u8_default_conversions!(ServiceState {
    Stopped,
    Starting,
    Running,
    Failed,
}, default = Failed);

/// A handle to a running kernel service.
pub struct ServiceHandle {
    pub service: InitService,
    pub task_id: TaskId,
    pub name: &'static str,
    state: AtomicU8,
}

impl ServiceHandle {
    fn new(service: InitService, task_id: TaskId, name: &'static str) -> Self {
        Self {
            service,
            task_id,
            name,
            state: AtomicU8::new(ServiceState::Running.to_u8()),
        }
    }

    pub fn state(&self) -> ServiceState {
        ServiceState::from_u8(self.state.load(Ordering::Relaxed)).expect("invalid service state")
    }

    pub fn mark_failed(&self) {
        self.state
            .store(ServiceState::Failed.to_u8(), Ordering::Relaxed);
    }

    pub fn mark_stopped(&self) {
        self.state
            .store(ServiceState::Stopped.to_u8(), Ordering::Relaxed);
    }
}

lazy_static! {
    static ref SERVICE_TABLE: IrqSafeMutex<Vec<ServiceHandle>> = IrqSafeMutex::new(Vec::new());
}

static SERVICES_SPAWNED: AtomicU8 = AtomicU8::new(0);

/// Spawn all kernel init services as kernel tasks.
///
/// Must be called after the task registry and scheduler are online (i.e. after
/// `StartupStage::SmpInit`).  Idempotent — calling it twice is a no-op.
pub fn spawn_init_services() {
    // Guard: only run once.
    if SERVICES_SPAWNED
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    // (service, name, priority, entry fn as raw addr)
    // Function pointer→usize cast is only valid at runtime, not const.
    let entries: [(InitService, &'static str, u8, u64); NUM_SERVICES] = [
        (
            InitService::Watchdog,
            "kthread/watchdog",
            200,
            service_entry_watchdog as *const () as usize as u64,
        ),
        (
            InitService::JournalCommit,
            "kthread/jcommit",
            128,
            service_entry_jcommit as *const () as usize as u64,
        ),
        (
            InitService::MemPressure,
            "kthread/mempressure",
            150,
            service_entry_mempressure as *const () as usize as u64,
        ),
        (
            InitService::NetworkPoll,
            "kthread/netpoll",
            160,
            service_entry_netpoll as *const () as usize as u64,
        ),
        (
            InitService::Telemetry,
            "kthread/telemetry",
            64,
            service_entry_telemetry as *const () as usize as u64,
        ),
    ];

    let mut table = SERVICE_TABLE.lock();
    for (idx, &(svc, name, prio, entry)) in entries.iter().enumerate() {
        let tid = TaskId(0x8000 + idx); // kernel TIDs start at 0x8000
        let task = KernelTask::new(tid, prio, 0, 0, 0x0, 0x0, entry);
        crate::kernel::task::register_task(task);
        table.push(ServiceHandle::new(svc, tid, name));
    }
}

/// Returns a snapshot of all service handles.
pub fn service_handles() -> Vec<(InitService, TaskId, &'static str, ServiceState)> {
    SERVICE_TABLE
        .lock()
        .iter()
        .map(|h| (h.service, h.task_id, h.name, h.state()))
        .collect()
}

/// Mark a service as failed (e.g. if its task exits unexpectedly).
pub fn mark_service_failed(svc: InitService) {
    let table = SERVICE_TABLE.lock();
    for h in table.iter() {
        if h.service == svc {
            h.mark_failed();
            return;
        }
    }
}

// ── Service entry points ──────────────────────────────────────────────────────
//
// These are the "main loops" for each kernel service, executed in the context
// of their respective kernel tasks.  Each loop calls the subsystem's primary
// tick/poll function; where the API is not yet wired the loop simply parks with
// a spin hint.

extern "C" fn service_entry_watchdog() -> ! {
    loop {
        // Watchdog tick: expires tasks that have exceeded their deadline.
        crate::kernel::watchdog::tick();
        core::hint::spin_loop();
    }
}

extern "C" fn service_entry_jcommit() -> ! {
    loop {
        // Flush any outstanding journal transactions to stable storage.
        #[cfg(feature = "vfs")]
        {
            let _ = crate::modules::vfs::journal::commit();
        }
        core::hint::spin_loop();
    }
}

extern "C" fn service_entry_mempressure() -> ! {
    loop {
        // Trim page caches and run OOM killer if memory is critically low.
        crate::kernel::pressure::on_pressure_tick();
        core::hint::spin_loop();
    }
}

extern "C" fn service_entry_netpoll() -> ! {
    loop {
        // Network stack: process queued RX frames and flush TX queues.
        #[cfg(feature = "networking")]
        {
            crate::modules::network::transport::poll_all();
        }
        core::hint::spin_loop();
    }
}

extern "C" fn service_entry_telemetry() -> ! {
    loop {
        // Telemetry sink: serialise kernel counters to the diagnostic ring.
        core::hint::spin_loop();
    }
}
