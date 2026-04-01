mod failover;
mod quarantine;
mod service;
mod slo;

pub(crate) use service::{
    service_registered_network_driver_io, service_specific_network_driver_io,
};
pub(crate) use slo::maybe_auto_switch_network_driver_on_slo;
