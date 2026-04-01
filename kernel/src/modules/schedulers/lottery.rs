use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

mod config;
pub use config::{
    lottery_initial_seed, lottery_lcg_increment, lottery_lcg_multiplier,
    lottery_min_tickets_per_task, lottery_runtime_config, lottery_tickets_per_priority_level,
    set_lottery_initial_seed, set_lottery_lcg_increment, set_lottery_lcg_multiplier,
    set_lottery_min_tickets_per_task, set_lottery_runtime_config,
    set_lottery_tickets_per_priority_level, LotteryRuntimeConfig,
};

const REPLAY_CAPACITY: usize = crate::generated_consts::SCHED_LOTTERY_REPLAY_TRACE_CAPACITY;

/// Lottery Scheduler.
/// A probabilistic scheduler where tasks hold "tickets".
/// The scheduler picks a random ticket to decide who runs next.
/// Access patterns approach the ticket distribution over time.

static LOTTERY_ADD_CALLS: AtomicU64 = AtomicU64::new(0);
static LOTTERY_REMOVE_CALLS: AtomicU64 = AtomicU64::new(0);
static LOTTERY_PICK_CALLS: AtomicU64 = AtomicU64::new(0);
static LOTTERY_PICK_EMPTY: AtomicU64 = AtomicU64::new(0);
static LOTTERY_FALLBACK_FIRST: AtomicU64 = AtomicU64::new(0);
static LOTTERY_REPLAY_SEQ: AtomicU64 = AtomicU64::new(0);
static LOTTERY_REPLAY_OVERWRITES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotteryReplayEvent {
    pub seq: u64,
    pub task_id: TaskId,
    pub winner_ticket: u64,
    pub total_tickets: u64,
    pub rng_state: u64,
}

impl LotteryReplayEvent {
    pub const EMPTY: Self = Self {
        seq: 0,
        task_id: TaskId(0),
        winner_ticket: 0,
        total_tickets: 0,
        rng_state: 0,
    };
}

static LOTTERY_REPLAY_RING: Mutex<[LotteryReplayEvent; REPLAY_CAPACITY]> =
    Mutex::new([LotteryReplayEvent::EMPTY; REPLAY_CAPACITY]);

#[derive(Debug, Clone, Copy)]
pub struct LotteryRuntimeStats {
    pub add_calls: u64,
    pub remove_calls: u64,
    pub pick_calls: u64,
    pub pick_empty: u64,
    pub fallback_first: u64,
    pub replay_latest_seq: u64,
    pub replay_overwrites: u64,
}

pub fn runtime_stats() -> LotteryRuntimeStats {
    LotteryRuntimeStats {
        add_calls: LOTTERY_ADD_CALLS.load(Ordering::Relaxed),
        remove_calls: LOTTERY_REMOVE_CALLS.load(Ordering::Relaxed),
        pick_calls: LOTTERY_PICK_CALLS.load(Ordering::Relaxed),
        pick_empty: LOTTERY_PICK_EMPTY.load(Ordering::Relaxed),
        fallback_first: LOTTERY_FALLBACK_FIRST.load(Ordering::Relaxed),
        replay_latest_seq: LOTTERY_REPLAY_SEQ.load(Ordering::Relaxed),
        replay_overwrites: LOTTERY_REPLAY_OVERWRITES.load(Ordering::Relaxed),
    }
}

fn record_replay_event(task_id: TaskId, winner_ticket: u64, total_tickets: u64, rng_state: u64) {
    let seq = LOTTERY_REPLAY_SEQ
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    let idx = (seq as usize) % REPLAY_CAPACITY;
    let mut ring = LOTTERY_REPLAY_RING.lock();
    if ring[idx].seq != 0 {
        LOTTERY_REPLAY_OVERWRITES.fetch_add(1, Ordering::Relaxed);
    }
    ring[idx] = LotteryReplayEvent {
        seq,
        task_id,
        winner_ticket,
        total_tickets,
        rng_state,
    };
}

pub fn latest_replay_event() -> Option<LotteryReplayEvent> {
    let seq = LOTTERY_REPLAY_SEQ.load(Ordering::Relaxed);
    if seq == 0 {
        return None;
    }
    let idx = (seq as usize) % REPLAY_CAPACITY;
    let ev = LOTTERY_REPLAY_RING.lock()[idx];
    if ev.seq == 0 {
        None
    } else {
        Some(ev)
    }
}

pub fn replay_event_count() -> usize {
    let seq = LOTTERY_REPLAY_SEQ.load(Ordering::Relaxed) as usize;
    core::cmp::min(seq, REPLAY_CAPACITY)
}

pub fn replay_recent_into(out: &mut [LotteryReplayEvent]) -> usize {
    if out.is_empty() {
        return 0;
    }
    let ring = LOTTERY_REPLAY_RING.lock();
    let total = replay_event_count();
    if total == 0 {
        return 0;
    }
    let n = core::cmp::min(out.len(), total);
    let oldest = if total == REPLAY_CAPACITY {
        (LOTTERY_REPLAY_SEQ.load(Ordering::Relaxed) as usize) % REPLAY_CAPACITY
    } else {
        0
    };
    let start = total.saturating_sub(n);
    let mut cursor = (oldest + start) % REPLAY_CAPACITY;
    let mut written = 0usize;
    while written < n {
        let ev = ring[cursor];
        if ev.seq != 0 {
            out[written] = ev;
            written += 1;
        }
        cursor = (cursor + 1) % REPLAY_CAPACITY;
    }
    written
}

pub struct Lottery {
    tickets: Vec<(TaskId, u64)>, // (TaskID, NumTickets)
    tasks: alloc::collections::BTreeMap<TaskId, KernelTask>,
    total_tickets: u64,
    rng_state: u64,
}

impl Lottery {
    pub fn new() -> Self {
        Self::with_seed(lottery_initial_seed())
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            tickets: Vec::new(),
            tasks: alloc::collections::BTreeMap::new(),
            total_tickets: 0,
            rng_state: seed,
        }
    }

    fn next_random(&mut self) -> u64 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(lottery_lcg_multiplier())
            .wrapping_add(lottery_lcg_increment());
        self.rng_state
    }

    #[inline(always)]
    fn ticket_count_for_priority(priority: u8) -> u64 {
        let per_level = lottery_tickets_per_priority_level();
        let min = lottery_min_tickets_per_task();
        let priority_boost = (u8::MAX - priority) as u64;
        priority_boost.saturating_mul(per_level).saturating_add(min)
    }
}

pub(crate) fn priority_ticket_count_for_contract(priority: u8) -> u64 {
    Lottery::ticket_count_for_priority(priority)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotteryReplayTask {
    pub id: TaskId,
    pub priority: u8,
}

impl LotteryReplayTask {
    #[inline(always)]
    pub const fn new(id: TaskId, priority: u8) -> Self {
        Self { id, priority }
    }
}

fn replay_kernel_task(def: LotteryReplayTask) -> KernelTask {
    KernelTask::new(def.id, def.priority, 0, 0, 0, 0, 0)
}

pub fn deterministic_replay_trace(
    seed: u64,
    tasks: &[LotteryReplayTask],
    picks: usize,
) -> Vec<Option<TaskId>> {
    let mut sched = Lottery::with_seed(seed);
    for &task in tasks {
        sched.add_task(replay_kernel_task(task));
    }
    let mut trace = Vec::with_capacity(picks);
    for _ in 0..picks {
        trace.push(sched.pick_next());
    }
    trace
}

impl Scheduler for Lottery {
    type TaskItem = KernelTask;

    fn init(&mut self) {}

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.tasks.get_mut(&task_id)
    }

    fn add_task(&mut self, task: Self::TaskItem) {
        LOTTERY_ADD_CALLS.fetch_add(1, Ordering::Relaxed);
        let tid = task.id;
        let ticket_count = Self::ticket_count_for_priority(task.priority);
        self.tickets.push((tid, ticket_count));
        self.total_tickets += ticket_count;
        self.tasks.insert(tid, task);
    }

    fn remove_task(&mut self, task_id: TaskId) {
        LOTTERY_REMOVE_CALLS.fetch_add(1, Ordering::Relaxed);
        self.tasks.remove(&task_id);
        if let Some(pos) = self.tickets.iter().position(|(id, _)| *id == task_id) {
            self.total_tickets -= self.tickets[pos].1;
            self.tickets.remove(pos);
        }
    }

    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem> {
        LOTTERY_REMOVE_CALLS.fetch_add(1, Ordering::Relaxed);
        let task = self.tasks.remove(&task_id)?;
        if let Some(pos) = self.tickets.iter().position(|(id, _)| *id == task_id) {
            self.total_tickets = self.total_tickets.saturating_sub(self.tickets[pos].1);
            self.tickets.remove(pos);
        }
        Some(task)
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        LOTTERY_PICK_CALLS.fetch_add(1, Ordering::Relaxed);
        if self.total_tickets == 0 {
            LOTTERY_PICK_EMPTY.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        // Pick a winning ticket
        let rng_state = self.next_random();
        let winning_ticket = rng_state % self.total_tickets;

        let mut current_ticket = 0;
        for (id, count) in &self.tickets {
            current_ticket += count;
            if current_ticket > winning_ticket {
                record_replay_event(*id, winning_ticket, self.total_tickets, rng_state);
                return Some(*id);
            }
        }

        // Should not happen if logic is correct, fallback to first
        LOTTERY_FALLBACK_FIRST.fetch_add(1, Ordering::Relaxed);
        let picked = self.tickets.first().map(|(id, _)| *id);
        if let Some(id) = picked {
            record_replay_event(id, winning_ticket, self.total_tickets, rng_state);
        }
        picked
    }

    fn tick(&mut self, _current: TaskId) -> SchedulerAction {
        // Reschedule on every tick to ensure probabilistic distribution?
        // Or only after a slice?
        // Lottery often uses time slices.
        crate::interfaces::SchedulerAction::Reschedule
    }
}

#[cfg(test)]
mod tests;
