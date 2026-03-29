/// Governance interface for power and thermal management.
pub trait Governance {
    fn suggest_frequency(&self, load_percent: u8) -> u32;
    fn on_state_change(&mut self, new_state: SystemState);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    Active,
    Idle,
    DeepSleep,
}
