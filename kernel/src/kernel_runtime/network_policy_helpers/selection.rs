#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) fn preferred_policy_for_driver(
    driver: hypercore::modules::drivers::ActiveNetworkDriver,
) -> hypercore::modules::drivers::NetworkDriverPolicy {
    match driver {
        hypercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            hypercore::modules::drivers::NetworkDriverPolicy::PreferVirtIo
        }
        hypercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            hypercore::modules::drivers::NetworkDriverPolicy::PreferE1000
        }
        hypercore::modules::drivers::ActiveNetworkDriver::None => {
            hypercore::modules::drivers::network_driver_policy()
        }
    }
}

#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) fn select_network_failover_target(
    current: hypercore::modules::drivers::ActiveNetworkDriver,
    has_virtio: bool,
    has_e1000: bool,
) -> hypercore::modules::drivers::ActiveNetworkDriver {
    match current {
        hypercore::modules::drivers::ActiveNetworkDriver::VirtIo if has_e1000 => {
            hypercore::modules::drivers::ActiveNetworkDriver::E1000
        }
        hypercore::modules::drivers::ActiveNetworkDriver::E1000 if has_virtio => {
            hypercore::modules::drivers::ActiveNetworkDriver::VirtIo
        }
        _ => hypercore::modules::drivers::ActiveNetworkDriver::None,
    }
}
