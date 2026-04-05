pub mod lifecycle;
pub mod registry;

pub use lifecycle::{
    DeviceMatch, DriverBindingRecord, DriverKitClass, DriverKitEvent, DriverKitEventQueue,
    DriverKitHealthSnapshot, DriverKitRecoveryPolicy, DriverLifecycleState, UserModeDriverContext,
};
pub use registry::DriverKitRegistry;


#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    struct DummyDriver {
        score: u32,
        started: bool,
    }

    impl DriverKitClass for DummyDriver {
        fn class_name(&self) -> &'static str {
            "dummy"
        }

        fn score(&self, _device: &DeviceMatch) -> u32 {
            self.score
        }

        fn start(&mut self, _context: &UserModeDriverContext) -> Result<(), String> {
            self.started = true;
            Ok(())
        }

        fn stop(&mut self) -> Result<(), String> {
            self.started = false;
            Ok(())
        }

        fn on_event(&mut self, _event: DriverKitEvent) -> Result<(), String> {
            Ok(())
        }
    }

    #[test_case]
    fn registry_selects_highest_score() {
        let mut registry = DriverKitRegistry::new();
        registry.register(Box::new(DummyDriver {
            score: 10,
            started: false,
        }));
        registry.register(Box::new(DummyDriver {
            score: 90,
            started: false,
        }));

        let device = DeviceMatch {
            vendor_id: 0x1234,
            device_id: 0x5678,
            class_code: 2,
            subclass: 0,
        };

        assert_eq!(registry.select_best(&device), Some(1));
    }

    #[test_case]
    fn event_queue_respects_capacity() {
        let mut queue = DriverKitEventQueue::new(1);
        assert!(queue.push(DriverKitEvent::Start));
        assert!(!queue.push(DriverKitEvent::Interrupt));
        assert_eq!(queue.pop(), Some(DriverKitEvent::Start));
    }

    #[test_case]
    fn registry_tracks_binding_lifecycle() {
        let mut registry = DriverKitRegistry::new();
        registry.register(Box::new(DummyDriver {
            score: 42,
            started: false,
        }));

        let device = DeviceMatch {
            vendor_id: 0x1111,
            device_id: 0x2222,
            class_code: 2,
            subclass: 0,
        };
        let binding = registry.bind_device(device).expect("binding should be created");
        assert_eq!(binding.state, DriverLifecycleState::Bound);

        let ctx = UserModeDriverContext::new();
        registry
            .start_binding(0, &ctx)
            .expect("binding start should succeed");
        assert_eq!(
            registry.binding_state(0),
            Some(DriverLifecycleState::Started)
        );

        registry.stop_binding(0).expect("binding stop should succeed");
        assert_eq!(
            registry.binding_state(0),
            Some(DriverLifecycleState::Stopped)
        );
    }

    #[test_case]
    fn fault_and_recovery_flow_works() {
        let mut registry = DriverKitRegistry::new();
        registry.register(Box::new(DummyDriver {
            score: 10,
            started: false,
        }));

        let device = DeviceMatch {
            vendor_id: 0xAAAA,
            device_id: 0xBBBB,
            class_code: 1,
            subclass: 1,
        };
        let _ = registry.bind_device_with_policy(device, DriverKitRecoveryPolicy::balanced());
        let ctx = UserModeDriverContext::new();
        registry
            .start_binding(0, &ctx)
            .expect("binding start should work");

        registry
            .mark_fault_at_tick(0, 100)
            .expect("fault mark should work");
        assert_eq!(
            registry.binding_state(0),
            Some(DriverLifecycleState::Faulted)
        );
        assert!(registry.recover_faulted_binding_at_tick(0, &ctx, 105).is_err());

        registry
            .recover_faulted_binding_at_tick(0, &ctx, 120)
            .expect("faulted binding should recover");
        assert_eq!(
            registry.binding_state(0),
            Some(DriverLifecycleState::Started)
        );
    }

    #[test_case]
    fn quarantine_policy_blocks_recovery() {
        let mut registry = DriverKitRegistry::new();
        registry.register(Box::new(DummyDriver {
            score: 10,
            started: false,
        }));

        let device = DeviceMatch {
            vendor_id: 0xCCCC,
            device_id: 0xDDDD,
            class_code: 1,
            subclass: 2,
        };
        let _ = registry.bind_device_with_policy(device, DriverKitRecoveryPolicy::conservative());
        let ctx = UserModeDriverContext::new();
        registry
            .start_binding(0, &ctx)
            .expect("binding start should work");

        registry.mark_fault(0).expect("fault mark should work");
        assert_eq!(
            registry.binding_state(0),
            Some(DriverLifecycleState::Quarantined)
        );
        assert!(registry.recover_faulted_binding(0, &ctx).is_err());
    }

    #[test_case]
    fn health_snapshot_reports_binding_state_counts() {
        let mut registry = DriverKitRegistry::new();
        registry.register(Box::new(DummyDriver {
            score: 10,
            started: false,
        }));

        let device = DeviceMatch {
            vendor_id: 0x1000,
            device_id: 0x2000,
            class_code: 3,
            subclass: 0,
        };
        let _ = registry.bind_device(device).expect("bind should work");
        let ctx = UserModeDriverContext::new();
        registry
            .start_binding(0, &ctx)
            .expect("start should succeed");

        let snap = registry.health_snapshot();
        assert_eq!(snap.class_count, 1);
        assert_eq!(snap.binding_count, 1);
        assert_eq!(snap.started_count, 1);
    }

    #[test_case]
    fn backoff_window_grows_with_retry_count() {
        let mut registry = DriverKitRegistry::new();
        registry.register(Box::new(DummyDriver {
            score: 55,
            started: false,
        }));

        let device = DeviceMatch {
            vendor_id: 0x9999,
            device_id: 0x4444,
            class_code: 1,
            subclass: 1,
        };
        let _ = registry.bind_device_with_policy(device, DriverKitRecoveryPolicy::balanced());

        registry
            .mark_fault_at_tick(0, 10)
            .expect("first fault should work");
        let first = registry
            .binding_next_recovery_tick(0)
            .expect("first recovery tick should exist");
        registry
            .mark_fault_at_tick(0, 20)
            .expect("second fault should work");
        let second = registry
            .binding_next_recovery_tick(0)
            .expect("second recovery tick should exist");
        assert!(second > first);
    }

    #[test_case]
    fn health_score_prefers_started_and_low_failure_states() {
        let healthy = DriverKitHealthSnapshot {
            class_count: 2,
            binding_count: 2,
            started_count: 2,
            faulted_count: 0,
            quarantined_count: 0,
            dispatch_success_count: 100,
            dispatch_failure_count: 1,
        };
        let degraded = DriverKitHealthSnapshot {
            class_count: 2,
            binding_count: 2,
            started_count: 0,
            faulted_count: 1,
            quarantined_count: 1,
            dispatch_success_count: 10,
            dispatch_failure_count: 20,
        };

        let healthy_score = DriverKitRegistry::health_score_from_snapshot(&healthy);
        let degraded_score = DriverKitRegistry::health_score_from_snapshot(&degraded);
        assert!(healthy_score > degraded_score);
        assert!(healthy_score >= 70);
    }
}
