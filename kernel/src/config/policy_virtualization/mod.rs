use super::*;

mod execution;
mod runtime;
mod scope;

const POLICY_SCOPE_FULLY_ENABLED: &str = "fully-enabled";
const POLICY_SCOPE_RUNTIME_LIMITED: &str = "runtime-limited";
const POLICY_SCOPE_COMPILETIME_LIMITED: &str = "compiletime-limited";
const POLICY_SCOPE_MIXED_LIMITS: &str = "mixed-limits";
const POLICY_SCOPE_FULLY_DISABLED: &str = "fully-disabled";
