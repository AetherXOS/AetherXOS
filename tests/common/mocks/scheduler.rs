use core::sync::atomic::{AtomicBool, AtomicUsize, AtomicU64, Ordering};

pub struct MockScheduler {
    pub running: AtomicBool,
    pub task_count: AtomicUsize,
    pub context_switches: AtomicUsize,
    pub total_runtime_ns: AtomicU64,
    pub current_priority: AtomicUsize,
}

impl MockScheduler {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            task_count: AtomicUsize::new(0),
            context_switches: AtomicUsize::new(0),
            total_runtime_ns: AtomicU64::new(0),
            current_priority: AtomicUsize::new(0),
        }
    }

    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn spawn_task(&self) -> usize {
        let id = self.task_count.fetch_add(1, Ordering::SeqCst);
        id
    }

    pub fn get_task_count(&self) -> usize {
        self.task_count.load(Ordering::SeqCst)
    }

    pub fn context_switch(&self) {
        self.context_switches.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_context_switch_count(&self) -> usize {
        self.context_switches.load(Ordering::SeqCst)
    }

    pub fn record_runtime(&self, ns: u64) {
        self.total_runtime_ns.fetch_add(ns, Ordering::SeqCst);
    }

    pub fn get_total_runtime_ns(&self) -> u64 {
        self.total_runtime_ns.load(Ordering::SeqCst)
    }

    pub fn set_priority(&self, priority: usize) {
        self.current_priority.store(priority, Ordering::SeqCst);
    }

    pub fn get_priority(&self) -> usize {
        self.current_priority.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.task_count.store(0, Ordering::SeqCst);
        self.context_switches.store(0, Ordering::SeqCst);
        self.total_runtime_ns.store(0, Ordering::SeqCst);
        self.current_priority.store(0, Ordering::SeqCst);
    }
}

impl Default for MockScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MockTask {
    pub id: usize,
    pub priority: usize,
    pub state: TaskState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

impl MockTask {
    pub fn new(id: usize, priority: usize) -> Self {
        Self {
            id,
            priority,
            state: TaskState::Ready,
        }
    }

    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }

    pub fn get_state(&self) -> TaskState {
        self.state
    }
}
