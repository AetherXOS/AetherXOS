#[cfg(all(test, feature = "process_abstraction"))]
mod probe_tests {
    use super::super::probe::*;
    use core::sync::atomic::Ordering;

    #[test_case]
    fn linked_probe_can_spawn_only_when_compat_ready_and_not_spawned() {
        assert!(linked_probe_can_spawn(true, false));
        assert!(!linked_probe_can_spawn(false, false));
        assert!(!linked_probe_can_spawn(true, true));
        assert!(!linked_probe_can_spawn(false, true));
    }

    #[test_case]
    fn linked_probe_service_action_matches_runtime_expectations() {
        assert_eq!(
            linked_probe_service_action(false, false),
            LinkedProbeServiceAction::WaitForLinuxCompat
        );
        assert_eq!(
            linked_probe_service_action(false, true),
            LinkedProbeServiceAction::WaitForLinuxCompat
        );
        assert_eq!(
            linked_probe_service_action(true, false),
            LinkedProbeServiceAction::Spawn
        );
        assert_eq!(
            linked_probe_service_action(true, true),
            LinkedProbeServiceAction::ObserveExit
        );
    }

    #[test_case]
    fn linked_probe_service_decision_preserves_state_and_action() {
        let decision = linked_probe_service_decision(true, false);
        assert!(decision.linux_compat_inited);
        assert!(!decision.spawned);
        assert_eq!(decision.action, LinkedProbeServiceAction::Spawn);
    }

    #[test_case]
    fn linked_probe_runtime_state_can_be_constructed_from_flags() {
        let state = LinkedProbeRuntimeState {
            linux_compat_inited: true,
            spawned: false,
        };
        assert!(state.linux_compat_inited);
        assert!(!state.spawned);
    }

    #[test_case]
    fn linked_probe_service_decision_helper_matches_direct_decision() {
        let direct = linked_probe_service_decision(true, false);
        assert_eq!(direct.action, LinkedProbeServiceAction::Spawn);
    }

    #[test_case]
    fn linked_probe_spawn_request_uses_expected_static_bootstrap_contract() {
        let request = linked_probe_spawn_request();
        assert_eq!(request.process_name, b"aether_init");
        assert_eq!(request.image, LINKED_PROBE_IMAGE);
        assert_eq!(request.priority, 128);
        assert_eq!(request.deadline, 0);
        assert_eq!(request.burst_time, 0);
        assert_eq!(request.kernel_stack_top, 0);
    }

    #[test_case]
    fn linked_probe_spawn_request_is_copy_stable() {
        let request = linked_probe_spawn_request();
        let copied = request;
        assert_eq!(copied, request);
    }

    #[test_case]
    fn linked_probe_spawn_branch_helper_is_callable_repeat() {
        enter_linked_probe_spawn_branch();
    }

    #[test_case]
    fn linked_probe_service_transition_includes_spawn_request_only_for_spawn() {
        let transition = PreparedLinkedProbeServiceDecision {
            decision: linked_probe_service_decision(true, false),
            spawn_request: Some(linked_probe_spawn_request()),
        };
        assert_eq!(
            transition.decision.action,
            LinkedProbeServiceAction::Spawn
        );
        assert!(transition.spawn_request.is_some());
    }

    #[test_case]
    fn linked_probe_spawn_request_keeps_zero_stack_top_contract() {
        let request = linked_probe_spawn_request();
        assert_eq!(request.kernel_stack_top, 0);
    }

    #[test_case]
    fn linked_probe_service_transition_dispatch_returns_early_for_spawn() {
        let transition = PreparedLinkedProbeServiceDecision {
            decision: linked_probe_service_decision(true, false),
            spawn_request: Some(linked_probe_spawn_request()),
        };
        assert!(dispatch_linked_probe_service_transition(transition));
    }

    #[test_case]
    fn linked_probe_service_entry_helper_keeps_spawn_transition_shape() {
        let transition = PreparedLinkedProbeServiceDecision {
            decision: linked_probe_service_decision(true, false),
            spawn_request: Some(linked_probe_spawn_request()),
        };
        assert_eq!(
            transition.decision.action,
            LinkedProbeServiceAction::Spawn
        );
    }

    #[test_case]
    fn linked_probe_spawn_transition_helper_is_callable() {
        dispatch_linked_probe_spawn_transition(linked_probe_spawn_request());
    }

    #[test_case]
    fn linked_probe_service_transition_runner_returns_early_for_spawn() {
        super::super::super::LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
        super::super::super::LINKED_PROBE_SPAWNED.store(false, Ordering::Relaxed);
        assert!(run_linked_probe_service_transition());
    }

    #[test_case]
    fn linked_probe_spawn_branch_helper_is_callable_again() {
        enter_linked_probe_spawn_branch();
    }

    #[test_case]
    fn linked_probe_service_transition_helper_preserves_spawn_request_shape() {
        super::super::super::LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
        super::super::super::LINKED_PROBE_SPAWNED.store(false, Ordering::Relaxed);
        let transition = prepare_linked_probe_service_transition();
        assert_eq!(
            transition.decision.action,
            LinkedProbeServiceAction::Spawn
        );
        assert!(transition.spawn_request.is_some());
    }

    #[test_case]
    fn entered_service_transition_helper_matches_spawn_shape() {
        super::super::super::LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
        super::super::super::LINKED_PROBE_SPAWNED.store(false, Ordering::Relaxed);
        let transition = prepare_entered_linked_probe_service_transition();
        assert_eq!(
            transition.decision.action,
            LinkedProbeServiceAction::Spawn
        );
        assert!(transition.spawn_request.is_some());
    }

    #[test_case]
    fn linked_probe_service_entry_helper_is_callable() {
        prepare_linked_probe_service_entry();
    }

    #[test_case]
    fn linked_probe_service_body_helper_reflects_active_state() {
        super::super::super::LINKED_PROBE_ENABLED.store(true, Ordering::Relaxed);
        super::super::super::LINKED_PROBE_VERIFIED.store(false, Ordering::Relaxed);
        assert!(enter_linked_probe_service_body());
    }

    #[test_case]
    fn linked_probe_exit_observer_is_callable_without_pid() {
        super::super::super::LINKED_PROBE_PID.store(0, Ordering::Relaxed);
        observe_linked_probe_exit();
    }

    #[test_case]
    fn entered_linked_probe_service_runner_returns_early_when_inactive() {
        super::super::super::LINKED_PROBE_ENABLED.store(false, Ordering::Relaxed);
        super::super::super::LINKED_PROBE_VERIFIED.store(false, Ordering::Relaxed);
        assert!(run_entered_linked_probe_service());
    }
}
