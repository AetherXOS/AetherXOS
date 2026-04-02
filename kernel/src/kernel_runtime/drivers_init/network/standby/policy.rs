pub(super) fn resolve_standby_policy(
    policy: aethercore::modules::drivers::NetworkDriverPolicy,
) -> Option<(
    aethercore::modules::drivers::ActiveNetworkDriver,
    aethercore::modules::drivers::NetworkDriverPolicy,
)> {
    let fallback_kind = aethercore::modules::drivers::probe_policy_fallback_kind(policy);
    let fallback_policy = match fallback_kind {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            Some(aethercore::modules::drivers::NetworkDriverPolicy::VirtIoOnly)
        }
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            Some(aethercore::modules::drivers::NetworkDriverPolicy::E1000Only)
        }
        aethercore::modules::drivers::ActiveNetworkDriver::None => None,
    }?;

    Some((fallback_kind, fallback_policy))
}

pub(super) fn standby_driver_ready(
    fallback_kind: aethercore::modules::drivers::ActiveNetworkDriver,
) -> bool {
    match fallback_kind {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            aethercore::modules::drivers::has_virtio_runtime_driver()
        }
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            aethercore::modules::drivers::has_e1000_runtime_driver()
        }
        aethercore::modules::drivers::ActiveNetworkDriver::None => false,
    }
}
