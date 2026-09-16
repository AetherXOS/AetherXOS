# Architecture Revision Plan - Observability, Modularity, and Stability

## Goals
- Make log, trace, debug, and error detection precise enough to identify failures at the point of origin.
- Keep the full feature matrix buildable under `--all-features`.
- Reduce cross-module visibility friction by publishing only the minimal necessary crate-internal APIs.
- Remove repeated logic by collapsing duplicated backend-specific patterns into small local adapters.
- Separate stable architecture-neutral contracts from feature-specific or platform-specific implementations.

## Why This Order
- Observability comes first because without precise traces we only guess where a failure started.
- Full-feature build stability comes second because refactoring on top of a broken matrix hides real regressions.
- Modularization comes third because duplication and tight coupling are what keep reintroducing the same bugs.
- Warning cleanup comes last because warnings are only meaningful after the underlying behavior is stable.

## Immediate Work
- Finish centralizing observability calls through the macro layer.
- Keep launch, boot, and graphics handoff traces consistent and structured.
- Continue tightening linux-compat dispatch paths so they use explicit imports instead of ambiguous glob exports.
- Remove remaining duplicated helper code in driver, syscall, and capability paths by introducing narrow internal helper APIs.
 
### Progress (applied)
- Centralized core audit/log helper: moved audit wrapper into `kernel_runtime::integration_utils` and replaced duplicated in-file helpers in `service_integration`.
- Reduced warning noise across the codebase: removed multiple unused imports, unnecessary parentheses in driver code, and cleaned several unused variables/`mut` markers.
- Extracted small shared logging helpers and used them across telemetry/driver integration sites.

These changes were verified with `cargo check -p aether-x-os --lib --all-features` and the build is green.

### Next Steps (short term)
1. Extract more integration helpers from `service_integration.rs` (audit, policy reporting) into `integration_utils::logging`.
2. Add structured trace points for remediation and network SLO flows where currently only unstructured logs exist.
3. Sweep remaining warnings and convert noisy debug lines into structured observability traces.
4. Test warnings sweep: remove unused imports and unnecessary `mut` in test helpers; convert noisy assertions that rely on always-true comparisons.
5. Prepare a small follow-up PR grouping:
	- `integration_utils` helpers and tests
	- telemetry/driver logging normalization
	- linux-compat dispatch clarity

Progress notes: The codebase builds and tests under `--all-features`. Remaining work is primarily cleanup and moving more observability points into structured traces rather than free-form logs.

## Structural Follow-Ups
- Replace scattered backend-specific capability locking code with one internal capability-store interface.
- Normalize paging and syscall argument types at the abstraction boundary instead of at arbitrary call sites.
- Keep kernel-runtime networking remediation and telemetry modules aligned on the same exported stats types.
- Split “policy” from “mechanism” in kernel-runtime and modules so policy code depends on small contracts rather than implementation details.
- Replace repeated feature-gated branching with thin adapters at the module edge, not deep inside business logic.
- Consolidate architecture-neutral traits in `interfaces/` and keep architecture-specific code constrained to `hal/`.
- Remove duplicated test-only helpers by extracting shared fixtures into one place per subsystem.

## Validation Strategy
- Run `cargo check -p aether-x-os --lib --all-features` after each structural change.
- Run `cargo test -p aether-x-os --lib --all-features` once the build is clean.
- Prefer narrow, local traces over broad logging so failures can be triaged quickly.
- Keep refactors small enough that each step is reversible and independently buildable.

## Current Priority Order
1. Full-feature compilation stability.
2. Observability completeness for boot, launch, and graphics handoff.
3. Modularization and dependency reduction across `kernel_runtime`, `modules`, and `hal`.
4. Cleanup of remaining warnings after correctness is stable.

## Refactor Tracks
### 1. Contract First
- Move cross-cutting behavior behind interfaces in `interfaces/`.
- Keep subsystems talking through typed, minimal contracts instead of direct module-to-module calls.

### 2. Edge Adapters
- Put feature-gated or platform-specific code at the edges.
- Use one small adapter per backend instead of repeating `#[cfg]` branches across call sites.

### 3. Shared Helpers
- Extract repeated conversion, logging, and policy code into local helpers.
- Prefer one shared helper per subsystem over duplicated inline logic.

### 4. Traceability
- Add one structured trace per major handoff boundary.
- Emit enough metadata to correlate boot, launch, graphics, networking, and remediation paths.
