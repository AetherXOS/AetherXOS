# Production ABI + Performance Roadmap

## Goal
Establish a strict, repeatable production gate for Linux ABI compatibility and performance engineering quality.

## Milestone M1 (Current Sprint)
- Enforce strict preflight gating on:
  - Linux ABI trend regression
  - Linux ABI workload catalog pass-rate/regression
  - Linux ABI semantic matrix overall status
  - Performance engineering report strict thresholds
- Keep host-target compile green for xtask.

## Milestone M2
- Split remaining large modules:
  - release preflight ABI/report surfaces into dedicated submodules
  - linux_app_compat runtime probe/report serialization into focused files
- Reduce coupling between release checks and report generation.

## Milestone M3
- Add threshold governance:
  - versioned perf threshold presets (ci/dev/release)
  - explicit waiver expiration metadata and reason linting
- Add ABI trend guardrails:
  - per-family minimum coverage floor
  - unsupported syscall budget policy by tier

## Milestone M4
- Add CI quality gates:
  - enforce strict preflight on candidate/release branches
  - publish ABI/perf artifacts as mandatory CI bundle evidence
  - fail PR if regression appears without approved waiver

## Exit Criteria
- strict preflight passes with no waiver on mainline release candidates
- perf report score and normalized gate score stay above configured floors
- ABI trend dashboard reports no regression for release cut window
- workload catalog pass-rate remains above strict threshold
