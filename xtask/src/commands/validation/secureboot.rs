use anyhow::Result;
use crate::cli::SecurebootAction;

pub mod models;
pub mod mok;
pub mod ovmf;
pub mod pcr;
pub mod sbat;
pub mod sign;

pub(crate) const REPORT_SCHEMA_VERSION: u32 = 1;

/// Entry point for `cargo run -p xtask -- secureboot <action>`.
pub fn execute(action: &SecurebootAction) -> Result<()> {
    match action {
        SecurebootAction::Sign {
            dry_run,
            strict_verify,
        } => sign::sign(*dry_run, *strict_verify),
        SecurebootAction::SbatValidate { strict } => sbat::sbat_validate(*strict),
        SecurebootAction::PcrReport => pcr::pcr_report(),
        SecurebootAction::MokPlan => mok::mok_plan(),
        SecurebootAction::OvmfMatrix { dry_run } => ovmf::ovmf_matrix(*dry_run),
    }
}
