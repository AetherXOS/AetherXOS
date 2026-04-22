use crate::utils::report;

use super::constants;
use super::helpers::rate;
use super::models::{Layer, LayerPercentages, Scorecard, Totals, TotalsOut};

pub(super) struct ScoreBundle {
    pub(super) scorecard: Scorecard,
    pub(super) pass_rate: f64,
    pub(super) ci_ok: bool,
}

pub(super) fn build_score_bundle(
    profile: &str,
    ci: bool,
    totals: &Totals,
    host: &Layer,
    integration: &Layer,
    compat: &Layer,
    kernel: &Layer,
    qemu_layer: &Layer,
    include_qemu: bool,
) -> ScoreBundle {
    let total = totals.passed + totals.failed + totals.skipped;
    let executed = totals.passed + totals.failed;
    let pass_rate = if executed == 0 {
        100.0
    } else {
        ((totals.passed as f64 / executed as f64) * 1000.0).round() / 10.0
    };

    let host_rate = rate(host);
    let integration_rate = rate(integration);
    let compat_rate = rate(compat);
    let kernel_rate = rate(kernel);
    let qemu_rate = if include_qemu { rate(qemu_layer) } else { 100.0 };
    let overall = ((host_rate * constants::SCORE_WEIGHT_HOST)
        + (integration_rate * constants::SCORE_WEIGHT_INTEGRATION)
        + (compat_rate * constants::SCORE_WEIGHT_RUNTIME_PROBE)
        + (kernel_rate * constants::SCORE_WEIGHT_KERNEL_GATE)
        + (qemu_rate * constants::SCORE_WEIGHT_QEMU_GATE))
        .round();

    let ci_ok = totals.failed == 0;
    let scorecard = Scorecard {
        generated_utc: report::utc_now_iso(),
        profile: profile.to_string(),
        ci_enforce: ci,
        totals: TotalsOut {
            passed: totals.passed,
            failed: totals.failed,
            skipped: totals.skipped,
            total,
            pass_rate_pct: pass_rate,
        },
        layer_percentages: LayerPercentages {
            host_smoke_pass_rate_pct: host_rate,
            app_integration_pass_rate_pct: integration_rate,
            runtime_probe_pass_rate_pct: compat_rate,
            kernel_gate_pass_rate_pct: kernel_rate,
            qemu_gate_pass_rate_pct: qemu_rate,
            overall_compatibility_index_pct: overall,
            ci_policy_ok: ci_ok,
        },
    };

    ScoreBundle {
        scorecard,
        pass_rate,
        ci_ok,
    }
}
