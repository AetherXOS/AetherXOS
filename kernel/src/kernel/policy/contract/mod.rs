mod self_test;
mod validation;

pub use self::self_test::run_runtime_policy_contract_self_test;
#[cfg(test)]
pub(crate) use self::validation::runtime_policy_snapshot_contract_holds;
