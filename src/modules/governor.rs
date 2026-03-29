use crate::interfaces::Governance;
use crate::interfaces::SystemState;

#[derive(Default)]
pub struct PerformanceGovernor;
impl Governance for PerformanceGovernor {
    fn suggest_frequency(&self, _load_percent: u8) -> u32 {
        100
    } // Always high
    fn on_state_change(&mut self, _new_state: SystemState) {}
}

#[derive(Default)]
pub struct PowersaveGovernor;
impl Governance for PowersaveGovernor {
    fn suggest_frequency(&self, _load_percent: u8) -> u32 {
        10
    } // Always low
    fn on_state_change(&mut self, _new_state: SystemState) {}
}

#[derive(Default)]
pub struct ConservativeGovernor;
impl Governance for ConservativeGovernor {
    fn suggest_frequency(&self, load_percent: u8) -> u32 {
        if load_percent > 80 {
            100
        } else {
            20
        }
    }
    fn on_state_change(&mut self, _new_state: SystemState) {}
}

#[derive(Default)]
pub struct OnDemandGovernor;
impl Governance for OnDemandGovernor {
    fn suggest_frequency(&self, load_percent: u8) -> u32 {
        u32::from(load_percent) // Directly proportional to load
    }
    fn on_state_change(&mut self, _new_state: SystemState) {}
}
