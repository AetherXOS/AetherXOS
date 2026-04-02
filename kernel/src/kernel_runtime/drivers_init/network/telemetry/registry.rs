use super::super::super::super::*;

pub(super) fn log_driver_runtime_registry() {
    let registry = aethercore::modules::drivers::runtime_registry_snapshot();
    aethercore::klog_info!(
        "Driver runtime registry: virtio={} e1000={} register={} unregister={} hotplug_attach={} hotplug_detach={} last_attach={:?} last_detach={:?} events={} overwrites={} last_event={:?}",
        registry.has_virtio,
        registry.has_e1000,
        registry.register_calls,
        registry.unregister_calls,
        registry.hotplug_attach_calls,
        registry.hotplug_detach_calls,
        registry.last_attach,
        registry.last_detach,
        registry.event_count,
        registry.event_overwrites,
        registry.last_event
    );

    let mut recent_events = [aethercore::modules::drivers::DriverRuntimeEvent {
        seq: 0,
        kind: aethercore::modules::drivers::DriverRuntimeEventKind::Registered,
        driver: aethercore::modules::drivers::ActiveNetworkDriver::None,
    }; 4];
    let recent_count = aethercore::modules::drivers::runtime_registry_events(&mut recent_events);
    if recent_count > 0 {
        for event in recent_events.iter().take(recent_count) {
            aethercore::klog_info!(
                "Driver runtime event: seq={} kind={:?} driver={:?}",
                event.seq,
                event.kind,
                event.driver
            );
        }
    }
}
