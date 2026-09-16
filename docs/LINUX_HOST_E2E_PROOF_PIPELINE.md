# Linux Host E2E Proof Pipeline

This pipeline is Linux host only and validates seeded package/runtime closure with boot + userspace evidence.

## Scope

- Builds seeded ISO and initramfs artifacts.
- Boots with qemu smoke to capture early userspace markers.
- Executes strict linux-app-compat with package-stack and fs-stack requirements.
- Emits proof artifacts into reports/linux_host_e2e_proof.

## Command

```bash
scripts/linux_host_e2e_proof.sh
```

## Acceptance Criteria

- qemu smoke passes with hyper_init, pivot-root, and apt-seed markers.
- strict linux-app-compat passes with zero failures.
- reports/linux_app_runtime_probe_report.json contains package-stack required probes as true.
- reports/linux_app_compat_validation_scorecard.json shows ci_policy_ok true.
- reports/linux_host_e2e_proof/latest.json status is pass.
