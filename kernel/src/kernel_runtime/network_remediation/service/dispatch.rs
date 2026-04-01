use super::super::failover::try_network_failover_for_io_health;
use super::super::quarantine::{is_driver_quarantined, quarantine_driver};
use super::rebind::{rebind_e1000_driver, rebind_virtio_driver};
use super::state::{e1000_state, virtio_state, DriverIoHealthContext, DriverIoHealthState};
use crate::kernel_runtime::network_policy_helpers::{
    decide_network_io_health_action, NetworkIoHealthDecisionContext,
};
use crate::kernel_runtime::networking::NETWORK_DRIVER_QUARANTINE_REBIND_FAILURES;

pub(super) fn service_specific_network_driver_io(
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
) -> bool {
    if driver != hypercore::modules::drivers::ActiveNetworkDriver::None
        && is_driver_quarantined(driver)
    {
        let _ = try_network_failover_for_io_health(driver, "driver-quarantined");
        return false;
    }

    match driver {
        hypercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            let Some(serviced) =
                hypercore::modules::drivers::with_virtio_runtime_driver_mut(|runtime_driver| {
                    service_virtio_driver_io(runtime_driver, driver);
                    true
                })
            else {
                return hypercore::modules::drivers::has_virtio_runtime_driver();
            };
            serviced
        }
        hypercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            let Some(serviced) =
                hypercore::modules::drivers::with_e1000_runtime_driver_mut(|runtime_driver| {
                    service_e1000_driver_io(runtime_driver, driver);
                    true
                })
            else {
                return hypercore::modules::drivers::has_e1000_runtime_driver();
            };
            serviced
        }
        hypercore::modules::drivers::ActiveNetworkDriver::None => false,
    }
}

pub(super) fn service_registered_network_driver_io() -> bool {
    let active = hypercore::modules::drivers::active_network_driver();
    service_specific_network_driver_io(active)
}

fn service_driver_io<T: hypercore::modules::drivers::DriverLifecycle>(
    runtime_driver: &mut T,
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
    state: DriverIoHealthState,
    rebind_driver: fn(&mut T) -> bool,
) {
    if hypercore::modules::drivers::DriverLifecycle::service_io(runtime_driver).is_err() {
        hypercore::modules::drivers::service_network_irq(driver);
        let status = hypercore::modules::drivers::DriverLifecycle::status(runtime_driver);
        let driver_failed = matches!(
            status.state,
            hypercore::modules::drivers::DriverState::Failed
        );
        let context = DriverIoHealthContext {
            driver_failed,
            io_error_streak: state.record_io_error(),
            rebind_failure_streak: state.rebind_failures(),
        };
        apply_io_health_action(
            runtime_driver,
            driver,
            state,
            decide_network_io_health_action(NetworkIoHealthDecisionContext {
                io_error_streak: context.io_error_streak,
                rebind_failure_streak: context.rebind_failure_streak,
                driver_failed: context.driver_failed,
            }),
            context,
            rebind_driver,
        );
    } else {
        state.clear_all();
    }
}

fn handle_failed_rebind(
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
    context: &DriverIoHealthContext,
    state: &DriverIoHealthState,
) {
    let failures = state.record_rebind_failure();
    if failures >= NETWORK_DRIVER_QUARANTINE_REBIND_FAILURES {
        quarantine_driver(driver, state.quarantine_reason, failures);
    }
    if matches!(
        decide_network_io_health_action(NetworkIoHealthDecisionContext {
            io_error_streak: 0,
            rebind_failure_streak: failures,
            driver_failed: context.driver_failed,
        }),
        hypercore::modules::drivers::NetworkIoHealthAction::TriggerFailover
    ) && try_network_failover_for_io_health(driver, "io-health-rebind-failed")
    {
        state.clear_rebind_failures();
    }
}

fn handle_failover_threshold(
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
    state: &DriverIoHealthState,
) {
    if try_network_failover_for_io_health(driver, "io-health-failover-threshold") {
        state.clear_all();
    }
}

fn apply_io_health_action<T>(
    runtime_driver: &mut T,
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
    state: DriverIoHealthState,
    action: hypercore::modules::drivers::NetworkIoHealthAction,
    context: DriverIoHealthContext,
    rebind_driver: fn(&mut T) -> bool,
) {
    match action {
        hypercore::modules::drivers::NetworkIoHealthAction::NoAction => {}
        hypercore::modules::drivers::NetworkIoHealthAction::AttemptRebind => {
            let rebind_ok = rebind_driver(runtime_driver);
            state.clear_io_errors();
            if rebind_ok {
                state.clear_rebind_failures();
            } else {
                handle_failed_rebind(driver, &context, &state);
            }
        }
        hypercore::modules::drivers::NetworkIoHealthAction::TriggerFailover => {
            handle_failover_threshold(driver, &state);
        }
    }
}

fn service_virtio_driver_io(
    runtime_driver: &mut hypercore::modules::drivers::VirtIoNet,
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
) {
    service_driver_io(runtime_driver, driver, virtio_state(), rebind_virtio_driver);
}

fn service_e1000_driver_io(
    runtime_driver: &mut hypercore::modules::drivers::E1000,
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
) {
    service_driver_io(runtime_driver, driver, e1000_state(), rebind_e1000_driver);
}
