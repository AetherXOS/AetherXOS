/// Cgroup PIDs Controller — limit the number of tasks in a cgroup.

/// PIDs controller state for one cgroup.
#[derive(Debug, Clone)]
pub struct PidsController {
    /// Maximum number of tasks (0 = unlimited).
    pub max: u64,
    /// Current task count.
    pub current: u64,
}

impl PidsController {
    pub fn new() -> Self {
        Self { max: 0, current: 0 }
    }

    /// Try to fork a new task. Returns false if the limit is reached.
    pub fn try_charge(&mut self) -> bool {
        if self.max > 0 && self.current >= self.max {
            return false;
        }
        self.current += 1;
        true
    }

    /// Task exited.
    pub fn uncharge(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    /// Set the maximum task count.
    pub fn set_max(&mut self, max: u64) {
        self.max = max;
    }
}
