# Scheduler Policy Contract (Library-Facing Service Classes)

This document defines the stable scheduler-policy contract consumed by library service loops.

## Scope

- Applies to runtime policy handoff from kernel scheduler pressure into library modules.
- Current integrations:
  - Network service loops: `modules::libnet::service_templates`.
  - Storage service loops: `modules::vfs::service_templates`.

## Priority Semantics

- Global task priority range is `0..=255`.
- Lower numeric value means higher priority.
- Contract checks run at boot via `kernel::scheduler_contract::run_scheduler_contract_self_test()`.
- Contract violations are fail-fast and logged with explicit `E3xxx` error codes.

## Pressure Snapshot Contract

- Source: `kernel::pressure::snapshot()`.
- Schema version: `CORE_PRESSURE_SCHEMA_VERSION = 2`.
- Snapshot includes:
  - Core pressure class: `Nominal | Elevated | High | Critical`.
  - Scheduler pressure class: `Nominal | Elevated | High | Critical`.
  - RT starvation signal and load-balance percentile signals.

## Mapping Contract

### Network (`libnet`)

- Entry point: `recommended_service_preset()`.
- Input: current `CorePressureSnapshot`.
- Output: `ServicePreset` (`ControlHeavy | ThroughputHeavy | PowerSave | LowLatency`).
- `scheduler_class == Critical` has override priority.
- `rt_starvation_alert == true` biases to `LowLatency`.

### Storage (`vfs`)

- Entry points:
  - `recommended_storage_preset()`
  - `recommended_io_policy()`
  - `apply_recommended_io_policy(&mut DiskFsLibrary)` (`vfs_disk_fs` feature)
- Output classes:
  - `StorageServicePreset` (`ThroughputHeavy | Balanced | LowLatency`)
  - `IoPolicy` (`Buffered | Unbuffered`)
- `scheduler_class == Critical` has override priority.
- `rt_starvation_alert == true` biases to `LowLatency`/`Unbuffered`.

## Compatibility Rules

- `CorePressureSnapshot` schema changes must bump `CORE_PRESSURE_SCHEMA_VERSION`.
- Existing class values must remain backward-compatible for one stable cycle.
- Library-side mapping functions must stay total (no panic path, no partial match).

## Operational Invariants

- Mapping hooks are pure and deterministic for the same input snapshot.
- No blocking I/O in policy mapping.
- Runtime loops may call hooks each cycle with bounded cost.
