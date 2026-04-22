#[path = "reports/coverage.rs"]
mod coverage;
#[path = "reports/feature.rs"]
mod feature;
#[path = "reports/readiness.rs"]
mod readiness;
#[path = "reports/maturity.rs"]
mod maturity;
#[path = "reports/userspace_abi.rs"]
mod userspace_abi;
#[path = "reports/virtualization.rs"]
mod virtualization;

pub use coverage::{coverage_audit, coverage_audit_with_telemetry};
pub use feature::feature_audit;
pub use maturity::{maturity_report, maturity_report_with_telemetry};
pub use readiness::{
	readiness_report,
	readiness_report_with_telemetry,
	release_gate_matrix,
	release_gate_matrix_with_telemetry,
};
pub use userspace_abi::{userspace_abi_report, userspace_abi_report_with_telemetry};
pub use virtualization::virtualization_readiness_report;