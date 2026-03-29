/// Cgroup Freezer Controller — suspend/resume all tasks in a cgroup.

/// Freezer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezerState {
    /// Tasks run normally.
    Thawed,
    /// All tasks are frozen (suspended).
    Frozen,
    /// Transition in progress.
    Freezing,
}

/// Freezer controller state for one cgroup.
#[derive(Debug, Clone)]
pub struct FreezerController {
    pub state: FreezerState,
    /// Number of freeze/thaw cycles.
    pub cycles: u64,
}

impl FreezerController {
    pub fn new() -> Self {
        Self {
            state: FreezerState::Thawed,
            cycles: 0,
        }
    }

    /// Freeze all tasks in this cgroup.
    pub fn freeze(&mut self) {
        if self.state != FreezerState::Frozen {
            self.state = FreezerState::Frozen;
            self.cycles += 1;
        }
    }

    /// Thaw (resume) all tasks.
    pub fn thaw(&mut self) {
        if self.state != FreezerState::Thawed {
            self.state = FreezerState::Thawed;
        }
    }

    /// Check if tasks should be frozen.
    pub fn is_frozen(&self) -> bool {
        self.state == FreezerState::Frozen
    }
}
