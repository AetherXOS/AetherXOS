# VFS Backend Operational Playbook (Fallback + Degraded Modes)

This playbook defines runtime operating modes for VFS backend incidents.

## Scope

- Applies to `DiskFsLibrary` consumers and mount-control operations.
- Covers fallback and degraded operation when latency/failure SLO breaches occur.

## Primary Signals

- Mount health SLO report: `modules::vfs::evaluate_mount_health_slo()`.
- VFS bridge/runtime counters:
  - mount/unmount failure rates
  - path validation rejects
  - disk read/write p99 latency ticks (when `vfs_telemetry` is enabled)

## Modes

1. `Normal`
- Preconditions:
  - no SLO breaches
- Action:
  - keep default buffered I/O policy

2. `Latency-Degraded`
- Trigger:
  - read or write p99 latency breach
- Action:
  - policy action: `PreferUnbufferedIo`
  - runtime applies `KernelConfig::set_vfs_enable_buffered_io(Some(false))`

3. `Mount-Churn-Degraded`
- Trigger:
  - mount/unmount failure-rate breach, path-validation breach, or capacity breach
- Action:
  - policy action: `ThrottleMountChurn` (advisory)
  - operators should reduce mount/unmount churn and investigate caller behavior

4. `Recovery`
- Trigger:
  - breach streak clears and sustained healthy samples
- Action:
  - policy action: `PreferBufferedIo`
  - runtime may restore buffered default via `KernelConfig::set_vfs_enable_buffered_io(Some(true))`

## Operator Runbook

1. Inspect VFS SLO telemetry logs (`[VFS SLO] ...`).
2. If `ThrottleMountChurn` appears:
   - investigate mount-path caller patterns
   - reduce dynamic mount namespace activity
3. If latency breaches persist:
   - keep unbuffered mode
   - isolate backend-specific hot paths (RAMFS vs disk-backed adapters)
4. After stable window with no breaches:
   - restore buffered mode
   - monitor p99 latencies and failure rates

## Rollback Strategy

1. Revert to buffered I/O default:
   - `KernelConfig::set_vfs_enable_buffered_io(Some(true))`
2. If regression resumes:
   - return to unbuffered mode
   - keep churn throttling advisory active

## Known Limits

- `ThrottleMountChurn` is currently advisory (no hard mount-rate limiter in core path).
- NFS/9P reconnect-aware recovery is still an open engineering item.
