# Roadmap Status

Last updated: 2026-03-06

Detailed execution roadmap: `docs/ROADMAP_DETAILED_2026_03_14.md`
Execution board: `docs/P0_P1_P2_EXECUTION_2026_03_14.md`

## P0 (Release Blockers)
- Status: Completed in current baseline
- Done:
  - Release preflight gates are active (`scripts/release_preflight.ps1`).
  - Syscall coverage gates are active for default + `linux_compat` profiles.
  - POSIX deep compile gate is active.
  - P0 orchestrator is active (`scripts/p0_readiness_gate.ps1`).
  - P0 gate supports strict reboot recovery thresholds (`-StrictRecoveryGate`).
  - P0 gate now detects dry-run/empty soak artifacts and skips reboot gate with explicit guidance unless artifacts are required.
  - Added strict routine wrapper for release acceptance (`scripts/p0_release_acceptance.ps1`) with real soak artifact enforcement by default.
  - Added combined nightly orchestration wrapper (`scripts/p0_p1_nightly.ps1`) for sequential P0 + P1 runs.
- Current metrics:
  - Default syscall coverage: implemented=258/258, partial=0, external=0, no=0.
  - `linux_compat` syscall coverage: implemented=257/257, partial=0, external=0, no=0.
- Remaining to close:
  - Run `scripts/p0_p1_nightly.ps1` on non-dry-run QEMU environments and persist artifacts in CI/nightly.

## P1 (Operational Hardening)
- Status: Started and active
- Done:
  - Added unified P1 operations gate (`scripts/p1_ops_gate.py`).
  - P1 gate runs preflight + soak/stress/chaos and emits combined report.
  - Optional QEMU soak + reboot recovery integration available in P1 gate.
  - Added trend/regression checks with automatic baseline support.
  - Added QEMU soak baseline regression and optional reboot baseline regression hooks.
  - Added dry-run capable QEMU path in P1 gate for CI/host validation (`--qemu-dry-run`).
  - Soak summary now includes percentile and failure-rate metrics (`p50/p95/failure_rate`).
  - Added long-duration release wrapper (`scripts/p1_release_acceptance.ps1`) with strict baseline/regression defaults.
  - Added rolling run history + trend-aware enforcement in P1 gate.
  - Added nightly runner profile (`scripts/p1_nightly.ps1`) for routine operations.
- Current command:
  - `python scripts/p1_ops_gate.py --skip-host-tests --soak-rounds 20 --soak-timeout-sec 300 --auto-baseline --update-baseline-on-success`
  - `powershell -ExecutionPolicy Bypass -File .\scripts\p1_release_acceptance.ps1 -SkipHostTests`
  - `powershell -ExecutionPolicy Bypass -File .\scripts\p1_nightly.ps1 -SkipHostTests`
- Remaining:
  - Execute non-dry-run nightly runs with required reboot baselines in CI and monitor first 2-week stability trend.

## P2 (Advanced Platform Features)
- Status: Started (planning and gap instrumentation)
- Done:
  - Added automated P2 gap inventory report (`scripts/p2_gap_report.py`) producing `reports/p2_gap/*`.
  - Added P2 marker regression gate (`scripts/p2_gap_gate.py`) with baseline-aware enforcement.
  - Integrated P2 gap gate into combined nightly orchestration (`scripts/p0_p1_nightly.ps1`).
  - Added A/B boot slot manager (`scripts/ab_boot_slots.py`) with stage/promote/mark-good/rollback/state flows.
  - Added A/B reboot recovery gate (`scripts/ab_boot_recovery_gate.py`) and wired into P1 ops gate (`--run-ab-recovery-gate`).
  - Boot image builder now supports optional A/B slot staging/promote (`scripts/build_boot_image.py --ab-slot ...`).
  - Added nightly A/B slot flip helper (`scripts/ab_nightly_slot_flip.py`) and optional integration in combined nightly orchestration.
- Open themes:
  - A/B boot + rollback flows.
  - Live update and config migration strategy.
  - Broader cross-architecture parity and production driver maturity.
  - Extended security policy hardening and formal evidence expansion.
