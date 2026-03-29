mod predicates;
mod self_test;

pub use self::predicates::{
    execution_profile_matches_status, expected_runtime_governor_class,
    virtualization_dispatch_contract_holds, virtualization_effective_execution_contract_holds,
    virtualization_effective_governor_contract_holds, virtualization_governor_bias_contract_holds,
    virtualization_runtime_mode_contract_holds,
};
pub use self::self_test::{run_virtualization_contract_self_test, VirtualizationContractReport};

#[cfg(test)]
mod tests;
