# Repository Structure Contract

This document defines where code should live, how new modules should be placed, and what to avoid to keep the repository understandable and maintainable over time.

## Goals

- Keep ownership boundaries obvious.
- Reduce path ambiguity between kernel, host tooling, and web dashboard code.
- Make onboarding and refactoring safer.
- Keep CI/test entry points predictable.

## Top-Level Ownership Map

- `kernel/`: production kernel code and architecture/runtime modules.
- `xtask/`: canonical automation entrypoint for build, check, validation, and reporting.
- `xagent/`: workspace agent crate used by automation/reporting flows.
- `dashboard/`: UI and e2e/testing stack for operational visibility.
- `build_cfg/`: config schema/types, validators, and codegen bridge.
- `config/`: policy/default/task manifests and command profiles.
- `tests/`: repository-level tests (outside kernel crate internals).
- `host_tools/`, `host_rust_tests/`: host-side utilities and host-only test logic.
- `fuzz/`: isolated fuzzing workspace.
- `formal/`: isolated formal verification workspace and assets.
- `docs/`: architecture decisions, runbooks, plans, and status docs.
- `artifacts/`: generated outputs (smoke runs, diagnostics, generated data).

## Placement Rules

1. Kernel runtime code goes under `kernel/src/**`.
2. Host-only helpers must not be placed under `kernel/src/**`; use `host_tools/` or `xtask/`.
3. New automation command flows should prefer `xtask/src/commands/**` over ad-hoc root scripts.
4. Dashboard-specific logic remains under `dashboard/**`; avoid cross-importing kernel internals there.
5. Fuzz/formal crates stay isolated from workspace members unless intentionally promoted.

## Naming and Organization Rules

1. Prefer domain-first folders over feature dumping:
- Good: `kernel/src/modules/net/...`
- Avoid: `kernel/src/misc/...` for unrelated additions
2. Keep module names explicit and stable (`linux_shim`, `allocator`, `scheduler`, `virt`).
3. New docs should be grouped under `docs/` by theme (`ops/`, `reports/`, roadmap/status files).

## Workspace Rules

1. Root workspace members are intentionally minimal:
- `.`
- `xtask`
- `xagent`
2. `fuzz/` and `formal/kani/` remain isolated with local `[workspace]` for direct invocation.
3. If adding a new workspace member, document rationale and impact in this file.

## Maintenance Rules

1. If a directory is deprecated, do not remove immediately:
- mark as deprecated in a short README
- migrate callers
- remove after references are clean
2. Keep root-level one-off logs/artifacts out of versioned source flow whenever possible.
3. Prefer one canonical path helper location (`xtask/src/utils/paths.rs`) for repository path derivations.

## Recommended Next Cleanups

1. Consolidate legacy root text logs into `reports/legacy/` or ignore them if not required.
2. Continue migrating script functionality from `scripts/` into `xtask` commands.
3. Add short README files for high-churn folders (`artifacts/`, `config/`, `build_cfg/`) describing expected contents.

## Change Control Checklist (for future PRs)

- Does the new file live in the correct domain folder?
- Is this host-only logic accidentally entering `kernel/src/**`?
- Is workspace membership needed, or should it be isolated?
- Is documentation updated if ownership/boundary changed?
