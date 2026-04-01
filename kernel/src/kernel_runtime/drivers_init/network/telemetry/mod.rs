mod dataplane;
mod registry;
mod remediation;

pub(super) fn log_network_runtime_dashboard() {
    dataplane::log_network_dataplane_dashboard();
    registry::log_driver_runtime_registry();
    remediation::log_network_remediation_dashboard();
}
