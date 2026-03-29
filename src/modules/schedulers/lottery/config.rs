#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotteryRuntimeConfig {
    pub initial_seed: u64,
    pub tickets_per_priority_level: u64,
    pub min_tickets_per_task: u64,
    pub lcg_multiplier: u64,
    pub lcg_increment: u64,
}

pub fn lottery_initial_seed() -> u64 {
    crate::config::KernelConfig::sched_lottery_initial_seed()
}

pub fn set_lottery_initial_seed(value: u64) {
    crate::config::KernelConfig::set_sched_lottery_initial_seed(Some(value));
}

pub fn lottery_tickets_per_priority_level() -> u64 {
    crate::config::KernelConfig::sched_lottery_tickets_per_priority_level()
}

pub fn set_lottery_tickets_per_priority_level(value: u64) {
    crate::config::KernelConfig::set_sched_lottery_tickets_per_priority_level(Some(value));
}

pub fn lottery_min_tickets_per_task() -> u64 {
    crate::config::KernelConfig::sched_lottery_min_tickets_per_task()
}

pub fn set_lottery_min_tickets_per_task(value: u64) {
    crate::config::KernelConfig::set_sched_lottery_min_tickets_per_task(Some(value));
}

pub fn lottery_lcg_multiplier() -> u64 {
    crate::config::KernelConfig::sched_lottery_lcg_multiplier()
}

pub fn set_lottery_lcg_multiplier(value: u64) {
    crate::config::KernelConfig::set_sched_lottery_lcg_multiplier(Some(value));
}

pub fn lottery_lcg_increment() -> u64 {
    crate::config::KernelConfig::sched_lottery_lcg_increment()
}

pub fn set_lottery_lcg_increment(value: u64) {
    crate::config::KernelConfig::set_sched_lottery_lcg_increment(Some(value));
}

pub fn lottery_runtime_config() -> LotteryRuntimeConfig {
    LotteryRuntimeConfig {
        initial_seed: lottery_initial_seed(),
        tickets_per_priority_level: lottery_tickets_per_priority_level(),
        min_tickets_per_task: lottery_min_tickets_per_task(),
        lcg_multiplier: lottery_lcg_multiplier(),
        lcg_increment: lottery_lcg_increment(),
    }
}

pub fn set_lottery_runtime_config(config: LotteryRuntimeConfig) {
    set_lottery_initial_seed(config.initial_seed);
    set_lottery_tickets_per_priority_level(config.tickets_per_priority_level);
    set_lottery_min_tickets_per_task(config.min_tickets_per_task);
    set_lottery_lcg_multiplier(config.lcg_multiplier);
    set_lottery_lcg_increment(config.lcg_increment);
}
