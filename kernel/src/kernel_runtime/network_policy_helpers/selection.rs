#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) fn preferred_policy_for_driver(
    driver: aethercore::modules::drivers::ActiveNetworkDriver,
) -> aethercore::modules::drivers::NetworkDriverPolicy {
    match driver {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            aethercore::modules::drivers::NetworkDriverPolicy::PreferVirtIo
        }
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            aethercore::modules::drivers::NetworkDriverPolicy::PreferE1000
        }
        aethercore::modules::drivers::ActiveNetworkDriver::None => {
            aethercore::modules::drivers::network_driver_policy()
        }
    }
}

#[cfg(all(feature = "drivers", feature = "networking"))]
pub(super) fn select_network_failover_target(
    current: aethercore::modules::drivers::ActiveNetworkDriver,
    has_virtio: bool,
    has_e1000: bool,
) -> aethercore::modules::drivers::ActiveNetworkDriver {
    match current {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo if has_e1000 => {
            aethercore::modules::drivers::ActiveNetworkDriver::E1000
        }
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 if has_virtio => {
            aethercore::modules::drivers::ActiveNetworkDriver::VirtIo
        }
        _ => aethercore::modules::drivers::ActiveNetworkDriver::None,
    }
}
