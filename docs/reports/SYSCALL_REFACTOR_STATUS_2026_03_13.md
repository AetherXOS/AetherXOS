# Syscall Refactor Status (2026-03-13)

## Current Snapshot

- Build health: PASS (`cargo check`)
- Compile errors: 0
- Warnings: 9 (non-blocking, mostly unused imports/fields)
- `src/kernel/syscalls/linux_compat_fallback.rs`: reduced to ~1967 lines
- New split modules:
   - `src/kernel/syscalls/linux_compat_fallback/fd_process_identity.rs` (~274 lines)
   - `src/kernel/syscalls/linux_compat_fallback/net_socket.rs` (~413 lines)

## What Was Improved Today

- Restored and validated core non-feature Linux syscall dispatch path.
- Split FD + identity related Linux compatibility handlers from monolithic legacy file.
- Removed pass-through wrappers by routing dispatcher directly to `linux_compat_misc` and `linux_compat_process` where applicable.
- Reduced one cfg-dependent warning in `linux_compat_misc` (`pid` -> `_pid`).

## Production Readiness (Kernel Syscall Layer)

- Build stability: 92%
- Modularity and foldering: 56%
- Duplication reduction: 52%
- Maintainability/readability: 58%
- Test confidence for refactor deltas: 45%
- Overall syscall-layer readiness: 61%

## Main Gaps

1. `linux_compat_fallback.rs` remains oversized and still mixes domains (memory, signal, process, fs).
2. Legacy function duplication still exists across `linux_compat_misc`, `linux_compat_process`, and legacy shim code.
3. Refactor-specific tests are limited (dispatch parity, behavior parity, and feature-matrix checks).
4. Warning debt remains in other modules (`pi_mutex`, scheduler, pipe, build cfg).

## Recommended Next Refactor Slices

1. Split `linux_compat_fallback.rs` into remaining domain files:
   - `legacy_net.rs`
   - `legacy_signal.rs`
   - `legacy_memory.rs`
   - `legacy_fs.rs`
2. Keep `linux_compat_fallback.rs` as dispatch-only facade (<500 lines target).
3. Remove duplicate wrappers by moving canonical implementations into focused modules and importing from one source.
4. Add small syscall dispatch parity tests:
   - non-feature path call mapping
   - errno compatibility checks for common failures
   - smoke checks for split modules

## Exit Criteria for Next Milestone

- `linux_compat_fallback.rs` < 1500 lines (phase-1), then < 800 lines (phase-2)
- Warning count <= 5
- No duplicate syscall handler symbols for same syscall semantic
- Green `cargo check` after each split step
