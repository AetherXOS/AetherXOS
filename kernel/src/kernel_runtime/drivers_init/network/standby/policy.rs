pub(super) fn resolve_standby_policy(
    policy: hypercore::modules::drivers::NetworkDriverPolicy,
) -> Option<(
    hypercore::modules::drivers::ActiveNetworkDriver,
    hypercore::modules::drivers::NetworkDriverPolicy,
)> {
    let fallback_kind = hypercore::modules::drivers::probe_policy_fallback_kind(policy);
    let fallback_policy = match fallback_kind {
        hypercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            Some(hypercore::modules::drivers::NetworkDriverPolicy::VirtIoOnly)
        }
        hypercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            Some(hypercore::modules::drivers::NetworkDriverPolicy::E1000Only)
        }
        hypercore::modules::drivers::ActiveNetworkDriver::None => None,
    }?;

    Some((fallback_kind, fallback_policy))
}

pub(super) fn standby_driver_ready(
    fallback_kind: hypercore::modules::drivers::ActiveNetworkDriver,
) -> bool {
    match fallback_kind {
        hypercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            hypercore::modules::drivers::has_virtio_runtime_driver()
        }
        hypercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            hypercore::modules::drivers::has_e1000_runtime_driver()
        }
        hypercore::modules::drivers::ActiveNetworkDriver::None => false,
    }
}
