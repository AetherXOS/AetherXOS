use serde::Serialize;

#[derive(Default)]
pub(super) struct Totals {
    pub(super) passed: usize,
    pub(super) failed: usize,
    pub(super) skipped: usize,
}

#[derive(Default, Serialize)]
pub(super) struct Layer {
    pub(super) passed: usize,
    pub(super) failed: usize,
    pub(super) skipped: usize,
    pub(super) total: usize,
}

#[derive(Serialize)]
pub(super) struct Scorecard {
    pub(super) generated_utc: String,
    pub(super) profile: String,
    pub(super) ci_enforce: bool,
    pub(super) totals: TotalsOut,
    pub(super) layer_percentages: LayerPercentages,
}

#[derive(Serialize)]
pub(super) struct TotalsOut {
    pub(super) passed: usize,
    pub(super) failed: usize,
    pub(super) skipped: usize,
    pub(super) total: usize,
    pub(super) pass_rate_pct: f64,
}

#[derive(Serialize)]
pub(super) struct LayerPercentages {
    pub(super) host_smoke_pass_rate_pct: f64,
    pub(super) app_integration_pass_rate_pct: f64,
    pub(super) runtime_probe_pass_rate_pct: f64,
    pub(super) kernel_gate_pass_rate_pct: f64,
    pub(super) qemu_gate_pass_rate_pct: f64,
    pub(super) overall_compatibility_index_pct: f64,
    pub(super) ci_policy_ok: bool,
}
