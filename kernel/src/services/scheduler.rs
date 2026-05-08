//! Scheduler service compatibility facade.

pub use crate::interfaces::scheduler::{Scheduler, SchedulerAction};

#[cfg(feature = "schedulers")]
pub use crate::modules::schedulers;

#[cfg(feature = "schedulers")]
pub use crate::modules::schedulers::config::SchedulerRuntimeConfig;

pub use crate::modules::selector::ActiveScheduler;
pub use crate::modules::selector::bootstrap_active_scheduler;