mod self_test;

pub use self::self_test::{run_scheduler_contract_self_test, SchedulerContractReport};

#[cfg(test)]
mod tests;
