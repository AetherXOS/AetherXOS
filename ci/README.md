CI notes: xtask-first workflow

This repository now uses a single Rust entrypoint for build/test/release automation:

```powershell
cargo run -p xtask -- --help
```

Quick start:

```powershell
# Host environment audit and auto-repair
cargo run -p xtask -- setup audit
cargo run -p xtask -- setup repair

# Full workspace bootstrap (audit + repair + build + smoke + dashboard)
cargo run -p xtask -- setup bootstrap
```

Build and run:

```powershell
# Build kernel/initramfs/ISO workflows
cargo run -p xtask -- build kernel
cargo run -p xtask -- build initramfs
cargo run -p xtask -- build iso
cargo run -p xtask -- build full

# QEMU run modes
cargo run -p xtask -- run smoke
cargo run -p xtask -- run live
```

Validation and quality gates:

```powershell
cargo run -p xtask -- test quality-gate
cargo run -p xtask -- test host
cargo run -p xtask -- test agent-contract
cargo run -p xtask -- test posix-conformance
cargo run -p xtask -- test driver-smoke
cargo run -p xtask -- test linux-app-compat
cargo run -p xtask -- test kernel-refactor-audit
```

Direct target-matrix quick check (PowerShell):

```powershell
pwsh ./ci/quality_targets.ps1 -Quiet
```

Optional policy gates:

```powershell
# Fail if non-HAL modules use arch-qualified HAL paths
pwsh ./ci/quality_targets.ps1 -FailOnArchBindings

# Explicitly mark HAL facade contract gate in logs
pwsh ./ci/quality_targets.ps1 -FailOnHalContract

# Fail when unsafe blocks do not have nearby Safety:/SAFETY: comment context
pwsh ./ci/quality_targets.ps1 -FailOnUnsafeWithoutSafetyComment
```

Release and nightly pipelines:

```powershell
cargo run -p xtask -- release preflight
cargo run -p xtask -- release candidate-gate
cargo run -p xtask -- release p0-gate
cargo run -p xtask -- release p0-acceptance
cargo run -p xtask -- release p1-nightly
cargo run -p xtask -- release p1-acceptance
cargo run -p xtask -- release p0p1-nightly
```

Linux ABI and syscall coverage:

```powershell
cargo run -p xtask -- linux-abi gap-inventory
cargo run -p xtask -- linux-abi readiness-score
cargo run -p xtask -- linux-abi errno-conformance
cargo run -p xtask -- linux-abi shim-errno-conformance
cargo run -p xtask -- linux-abi platform-readiness
cargo run -p xtask -- linux-abi gate
cargo run -p xtask -- linux-abi policy-drift
cargo run -p xtask -- linux-abi glibc-needs
cargo run -p xtask -- linux-abi p-tier-status
cargo run -p xtask -- linux-abi p2-gap-report
cargo run -p xtask -- linux-abi p2-gap-gate
cargo run -p xtask -- syscall-coverage
```

Operations and runtime tooling:

```powershell
# Soak and archive
cargo run -p xtask -- soak-test
cargo run -p xtask -- archive-nightly

# Secure Boot
cargo run -p xtask -- secureboot

# A/B boot slots
cargo run -p xtask -- ab-slot --help

# Crash and pressure diagnostics
cargo run -p xtask -- crash-recovery
cargo run -p xtask -- core-pressure

# Readiness dashboard data
cargo run -p xtask -- tier-status
```

Dashboard:

```powershell
cargo run -p xtask -- dashboard --help
```

Configuration registry now lives under:

- `config/aethercore.defaults.cjson`
- `config/aethercore.tasks.cjson`
- `config/aethercore.commands.cjson`
- `config/quick.actions.cjson`
- `config/hc_error_playbook.cjson`
- `config/aethercore.policy.cjson`
- `config/plugins/*.cjson`

Notes:
- Prefer xtask commands over direct script invocations.
- For Windows CI runners, install QEMU via Chocolatey: `choco install qemu`.
- For Linux/macOS, install `qemu-system-x86_64` via your package manager.

