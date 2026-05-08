#[cfg(test)]
mod one_shot_tests {
    
    #[test_case]
    fn one_shot_actions_only_attempt_when_not_already_done() {
        // Mock state or test logic
    }
}

#[cfg(all(test, feature = "process_abstraction"))]
mod process_abstraction_tests {
    use super::super::probe::*;
    
    #[test_case]
    fn linked_probe_main_loop_action_matches_gate_state() {
        assert_eq!(
            linked_probe_main_loop_action(false, false),
            LinkedProbeMainLoopAction::Skip
        );
        assert_eq!(
            linked_probe_main_loop_action(true, false),
            LinkedProbeMainLoopAction::Service
        );
    }
}
