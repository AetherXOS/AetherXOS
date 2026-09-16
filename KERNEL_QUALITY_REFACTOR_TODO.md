# Kernel Quality, Refactor, and Feature TODO

## Goals
- Raise maintainability by reducing monolith modules and implicit coupling.
- Improve safety by shrinking unsafe surface and making invariants explicit.
- Improve observability and diagnosability in interrupt/scheduler/runtime paths.
- Add targeted features that improve kernel operability without destabilizing ABI.

## Priority Matrix
- P0: Build integrity and correctness risks.
- P1: High-impact readability and architecture boundary clarity.
- P2: Feature and tooling improvements with medium risk.

## P0 - Correctness and Build Integrity
- [x] Replace tuple-style IRQ hottest-line snapshot with typed struct API.
  - Scope: `kernel/src/hal/aarch64/exception/irq_storm.rs`, `kernel/src/hal/aarch64/exception.rs`
  - Why: tuple fields are magic-position prone and fragile during extension.
  - Done when: no tuple destructuring remains in exception stats flow.
- [x] Harden scheduler switch contract per target/feature gates.
  - Scope: `kernel/src/kernel_runtime/interrupts/timer/{types.rs,schedule.rs}`
  - Why: cfg-gated fields can silently drift from constructors.
  - Done when: all `SwitchInfo` producers/consumers compile under both x86_64 and aarch64 checks.
- [x] Add compile-time assertions for descriptor table lengths in interrupt registration paths.
  - Scope: x86 IDT extended vectors and AArch64 platform IRQ descriptors.
  - Done when: mismatch fails at compile time where possible and at init-time otherwise.
  - Progress: x86 IDT extended route-count and AArch64 platform descriptor count assertions completed.

## P1 - Architecture and Code Quality
- [x] Split oversized process runtime modules into cohesive submodules.
  - Candidate split: API surface, lifecycle, image/loading, policy.
- [x] Remove stacked cfg attributes and replace with single `cfg(all(...))` style.
  - Scope: scheduler/interrupt/runtime paths first.
- [x] Introduce stronger typed wrappers for interrupt IDs and vector IDs.
  - Scope: `hal/common/irq_catalog.rs`, x86 IDT metadata, AArch64 platform IRQ tables.
- [x] Consolidate repeated interrupt logging format into shared helper macros/functions.
  - Scope: AArch64 `exception/irq.rs`, x86 `idt/dispatch.rs`.
- [x] Standardize module-level safety comments for `unsafe` blocks touching hardware.

## P2 - Features and Operability
- [x] Add per-IRQ moving-window telemetry snapshot endpoint via existing diagnostics surface.
- [x] Add interrupt storm mitigation mode profiles (`observe`, `throttle`, `panic-safe`) behind config flags.
  - Progress: `KernelConfig::interrupt_storm_profile()` now derives mode from existing core storm-protection + watchdog policy flags; AArch64 IRQ storm logic applies profile-aware thresholds and panic-safe throttling for non-critical lines during active storm windows.
- [x] Add scheduler decision trace sampling rate controls (not binary on/off).
  - Progress: `KernelConfig` now exposes sampling controls (`scheduler_trace_sample_rate`, `set_scheduler_trace_sample_rate`, `should_emit_scheduler_trace_sample`) and scheduler trace call-sites use sampled emission in timer scheduling, load balancing, and affinity-migration trace paths.
- [x] Extend QEMU smoke to assert interrupt-path health counters.
  - Progress: `xtask ops qemu smoke` now requires interrupt-health evidence from runtime logs (`x86_64 irq stats` or `AArch64 exception stats`) and enforces counter invariants before PASS.

## Testing and CI
- [x] Add target matrix checks: `x86_64-pc-windows-msvc`, `aarch64-unknown-none`.
- [x] Add lint gates for new unsafe blocks requiring a safety explanation comment.
  - Progress: `ci/quality_targets.ps1` now reports unsafe usages without nearby `Safety:`/`SAFETY:` comments and supports enforcement with `-FailOnUnsafeWithoutSafetyComment`.
- [x] Add no_std test gating policy and document `#[test_case]` usage boundaries.

## HAL Reliability Track
- [x] Reduce HAL facade duplication by routing serial emission through arch-neutral facade.
  - Scope: `kernel/src/hal/mod.rs`, `kernel/src/hal/serial.rs`
- [x] Start reducing HAL-external arch leakage in runtime paths by replacing direct x86 serial calls with facade calls.
  - Scope: `kernel/src/kernel/process_runtime/trace.rs`, `kernel/src/kernel_runtime/{heap.rs,interrupts/timer/{mod.rs,schedule.rs}}`
  - Progress: facade routing expanded across runtime/launch/loader/task/scheduler/vfs/syscall paths; HAL-external arch-binding scan count reduced from 206 to 0.
- [x] Centralize interrupt-controller enable unsafe path in common registration helper.
  - Scope: `kernel/src/hal/common/irq_registration.rs`, `kernel/src/hal/aarch64/platform/irq.rs`
- [x] Replace remaining HAL `static mut` hotspots with safe wrappers (`Mutex`/`Once`/`StaticCell`).
  - Scope: virtualization/APIC/SMP bootstrap globals.
  - Progress: x86 APIC calibrated-tick cache moved from `static mut` to `AtomicU32`; virtualization regions, x86_64 BSP/SMP/GDT stack statics, AArch64 bootstrap launch stacks, and AP CpuLocal/GDT storage moved to `UnsafeCell` wrappers/StaticCell helpers; HAL line-start `static mut` declarations reduced from 19 to 0.
- [x] Add HAL smoke checks for facade contract consistency (serial/interrupt/timer lifecycle).
  - Scope: host + target matrix checks.
  - Progress: `ci/quality_targets.ps1` now runs HAL facade contract checks (`hal/mod.rs` lifecycle methods, `hal/serial.rs` facade surface, runtime IRQ/IDT facade wiring) alongside dual-target compile checks.

## Completed In Current Iteration (Follow-up)
- x86 IDT dispatch no longer uses `static mut` dispatcher; safe mutex-backed storage is used.
- x86 IRQ dispatch metrics counters added (`total`, `timer`, `non_timer`, `dropped`, `dispatch_attempted`, `dispatch_handled`).
- AArch64 exception stats now include per-kind IRQ counters (`timer`, `serial`, `generic`, `tlb_shootdown`).
- AArch64 GIC EOI `unsafe` call centralized behind a dedicated helper.
- Host fallback test gating added for selected common modules using `cfg_attr(..., test_case)` + `cfg_attr(..., test)`.

## Execution Order
1. P0 snapshot typing and scheduler contract hardening.
2. P1 module decomposition and cfg normalization.
3. P2 feature increments with targeted smoke tests.

## Completed In Current Iteration
- IRQ hottest snapshot moved from tuple to typed struct.
- x86 IDT extended IRQ route count compile-time assertion added.
- `kernel/process_runtime.rs` decomposed with `mappings.rs`, `tls.rs`, `contract.rs`, and `trace.rs`.
- x86 early serial cfg repetition reduced via shared runtime trace helper.
- HAL ownership boundaries documented.
- CI target-matrix quick check script added and documented.
- CI target matrix script restored at `ci/quality_targets.ps1` and expanded with HAL-external arch-binding scan (`-FailOnArchBindings` optional gate).
- HAL stack storage statics in `hal/x86_64/mod.rs`, `hal/x86_64/smp.rs`, and `hal/aarch64/smp.rs` moved away from `static mut` to `UnsafeCell` wrappers.
- HAL virtualization and GDT globals (`hal/x86_64/virt.rs`, `hal/x86_64/virt/{vmx.rs,svm.rs}`, `hal/x86_64/gdt/{stacks.rs,mod.rs}`) migrated off `static mut` declarations.
- x86_64 SMP responsibilities split into submodules (`hal/x86_64/smp/storage.rs`, `hal/x86_64/smp/tlb.rs`) to isolate stack/cpu-local storage and TLB shootdown logic from orchestration flow.
- x86_64 virtualization region/state storage extracted to `hal/x86_64/virt/regions.rs`, reducing `virt.rs` responsibility to orchestration/capability flow.

## Production-Grade Kernel + XTask Roadmap (New)

### Kernel - Reliability and Safety (P0)
- [ ] Define panic/crash policy matrix per build profile (`dev`, `ci`, `release`) and enforce via compile-time cfg checks.
- [ ] Add lock-order and interrupt-context assertions for scheduler, VFS, and IPC hot paths.
- [ ] Expand memory corruption early-detection hooks (guard pages/canary checks) for kernel heap and stack-sensitive subsystems.
- [ ] Harden timekeeping and monotonic clock invariants under IRQ storm and TSC drift scenarios.

### Kernel - Isolation and Operability (P1)
- [x] Introduce capability/audit trace IDs for sensitive syscalls (mount, namespace, credential transitions).
- [ ] Add per-subsystem SLO counters (scheduler latency, IRQ tail latency, syscall error-rate windows).
- [ ] Add controlled degradation modes for low-memory and device-fault states (feature shedding without full panic).
- [ ] Define kernel ABI compatibility policy (stable/internal/experimental syscall classes).

### XTask - Production Automation (P0)
- [x] Add aggregated production acceptance scorecard generation in release status flow.
- [x] Add strict release preflight gate to fail when production acceptance scorecard is not green.
- [x] Add reproducible-build evidence command (artifact hash manifest + environment fingerprint).
- [x] Add machine-readable release evidence bundle command (single JSON index referencing all report artifacts).

### XTask - CI and Governance (P1)
- [x] Add trend-regression gate for P-tier score deltas (block release if required checks regress).
- [x] Add policy guard for forbidden warnings in critical kernel modules.
- [x] Add cross-platform host-tool verification command (Windows/Linux/macOS parity checks).
- [x] Add ABI drift alert command comparing current syscall tables to previous release baseline.
- [x] Add release diagnostics command that emits actionable remediation for failed scorecard/P-tier/evidence gates.
- [x] Add warning-audit command for critical kernel path warnings from build logs (strict/non-strict).
- [x] Add gate-fixup command to regenerate readiness/evidence/diagnostics artifacts in one pass.
- [x] Add CI bundle aggregator command that consolidates release gate health into a single report.

### Immediate Execution Queue
1. [x] Implement strict production preflight gate in `xtask release preflight`.
2. [x] Wire strict gate to generated scorecard JSON and emit actionable failure details.
3. [x] Validate host-target compile and smoke-run of updated xtask command path.
4. [x] Add strict trend-regression gate into nightly acceptance path (`release p1-nightly` / `candidate-gate`).
5. [x] Add ABI drift diff report command with previous release baseline manifest.

## Mega Production Backlog (Expanded)

### Program Governance and Delivery
- [ ] Define quarterly kernel reliability OKRs with owner, metric, and freeze criteria.
- [ ] Define release branching strategy (`main`, `release/*`, `hotfix/*`) and patch policy.
- [ ] Add architecture decision records (ADR) policy with mandatory ADR links in major PRs.
- [ ] Create risk register for kernel/runtime/security with mitigation owner and review cadence.
- [ ] Define API deprecation lifecycle for internal kernel interfaces.
- [ ] Add "production readiness review" checklist as release hard gate.
- [ ] Define support window and maintenance policy per release channel.
- [ ] Add backward-compatibility sign-off process for syscall ABI changes.

### Kernel Memory Management and VM
- [ ] Add page allocator fragmentation telemetry and threshold-based alerts.
- [ ] Add large page policy for kernel text/data where architecture supports it.
- [ ] Add page table consistency validator for debug/ci boots.
- [ ] Add TLB shootdown latency histograms and CPU outlier reporting.
- [ ] Add per-process virtual memory accounting with OOM candidate scoring.
- [x] Add kernel heap quarantine mode for use-after-free detection in debug profile.
- [ ] Add deterministic memory poisoning on free/alloc in test profile.
- [ ] Add copy-on-write stress harness for fork/clone heavy workloads.
- [ ] Add VM map audit command that validates VMA overlap/order invariants.
- [ ] Add guard region policy for critical kernel stacks and metadata slabs.

### Scheduler, Timekeeping, and CPU Topology
- [ ] Add runqueue fairness dashboards (per-core runnable imbalance over time).
- [ ] Add scheduler starvation detector with auto-dump of offending queues.
- [ ] Add CPU affinity conflict resolver for impossible mask requests.
- [ ] Add preemption latency tracing in timer/interrupt-heavy scenarios.
- [ ] Add monotonic clock cross-core drift monitor and correction strategy.
- [ ] Add tickless idle validation suite for power/perf regressions.
- [ ] Add cpuset-like policy layer for workload class isolation.
- [ ] Add scheduler policy conformance tests for all runtime policy modes.
- [ ] Add deterministic replay mode for scheduler decisions (test-only).

### Interrupts, Exceptions, and Fault Containment
- [ ] Add structured exception dump schema versioning for tooling stability.
- [ ] Add nested fault containment policy and panic-loop prevention guard.
- [ ] Add IRQ masking/unmasking audit log with bounded ring buffer retention.
- [ ] Add NMI-like emergency telemetry path with minimal lock dependency.
- [ ] Add per-vector interrupt budget and overload clamp strategy.
- [ ] Add fault injection hooks for exception table handlers in test builds.
- [ ] Add crash fingerprint generation for repeated fault clustering.

### Filesystem and Storage
- [ ] Add fsck-lite consistency pass for boot-time critical mounts.
- [ ] Add journaling durability test matrix for power-failure simulation.
- [ ] Add mount option validation matrix and unsafe-option denylist.
- [ ] Add inode/block leak detector in soak tests.
- [ ] Add fs metadata operation latency SLOs and periodic report.
- [ ] Add storage error-class taxonomy (`retryable`, `fatal`, `media`) in kernel logs.
- [ ] Add writeback pressure telemetry with eviction and stall diagnostics.
- [ ] Add overlayfs correctness regression suite for corner-case path semantics.

### Networking and IPC
- [ ] Add packet-path latency percentile tracking (P50/P95/P99).
- [ ] Add socket lifecycle leak detector in long-run soak jobs.
- [ ] Add connection storm backpressure policy and proof tests.
- [ ] Add queue depth control loops for loopback and virtual NIC paths.
- [ ] Add IPC channel deadlock detector and lock graph snapshots.
- [ ] Add rate-limit policy for noisy services in local IPC bus.
- [ ] Add deterministic net replay traces for regression reproducibility.
- [ ] Add configurable SYN flood resilience profile for compatibility mode.

### Security Hardening
- [ ] Add secure boot policy verification report as required release input.
- [ ] Add kernel image integrity chain audit with signature metadata checks.
- [ ] Add stack canary and control-flow hardening coverage report.
- [ ] Add privileged syscall access matrix and automated diff checks.
- [ ] Add namespace transition audit events with immutable trace IDs.
- [ ] Add anti-rollback metadata checks for boot artifacts.
- [ ] Add kernel secret handling policy (lifetime, zeroization, redaction).
- [ ] Add threat model document per subsystem and annual re-validation cycle.
- [ ] Add vuln response runbook with CVE triage SLA targets.

### Linux Compatibility and ABI Expansion
- [x] Add syscall semantic parity tests for errno/side-effect order guarantees.
- [x] Add time-related ABI parity suite (`clock_*`, `timer_*`, `ppoll`, `pselect`).
- [x] Add file descriptor edge-case parity suite (`dup`, `fcntl`, `close-on-exec`).
- [x] Add process/signal compatibility suite for multithreaded signal races.
- [x] Implement minimal functional `fanotify_init`/`fanotify_mark` syscall path (descriptor allocation + mark lifecycle) in non-`linux_compat` shim.
- [x] Add no-`posix_*` synthetic fallback path for `eventfd`, `timerfd_create`, and `memfd_create` (real fd token allocation instead of `ENOSYS`).
- [x] Make shim errno conformance source-driven and fix explicit `EFAULT` mappings for msg/socket/time/epoll shim helpers.
- [x] Close remaining compatibility syscall backlog: `bpf`, `inotify_init1`, `inotify_rm_watch`, `pidfd_open`, `signalfd4`.
- [x] Add ABI docs for intentionally unsupported syscalls and alternatives.
- [x] Add glibc/musl compatibility split report in CI bundle.
- [x] Add compatibility tier tags (`alpha`, `beta`, `ga`) per syscall family.
- [x] Add Linux userspace workload catalog and periodic pass-rate trend report.

### Observability and Diagnostics
- [ ] Add structured log schema (`event_id`, `component`, `severity`, `trace_id`).
- [ ] Add high-cardinality event safeguards and log sampling controls.
- [ ] Add kernel trace export format contract with versioned parsers.
- [ ] Add always-on lightweight health heartbeat for watchdog consumers.
- [ ] Add postmortem bundle command that packs dumps + key runtime reports.
- [ ] Add telemetry retention and truncation policy for constrained environments.
- [ ] Add boot-time milestone timeline report for startup regression tracking.

### Testing, Verification, and Quality Engineering
- [ ] Add deterministic nightly seed strategy for fuzz/property tests.
- [ ] Add chaos test suite for memory pressure + device I/O contention.
- [ ] Add long-haul soak profile (12h/24h) with pass/fail envelopes.
- [ ] Add kernel API contract tests for internal subsystem boundaries.
- [ ] Add architecture cross-check tests ensuring x86_64/aarch64 parity.
- [ ] Add Kani/Prusti/formal checks for critical lock-free data paths.
- [ ] Add failure-oriented tests for every release gate command in xtask.
- [ ] Add mutation testing pilot for syscall dispatch/error handling paths.
- [ ] Add flaky test quarantine policy with automatic re-enable workflow.

### Performance Engineering
- [ ] Add stable microbenchmark suite for scheduler, VM, and syscall hot paths.
- [x] Add regression threshold files in repo with explicit waiver flow.
- [ ] Add boot time budget per stage (`firmware`, `kernel init`, `userspace handoff`).
- [ ] Add interrupt tail-latency benchmark and capacity envelope per target.
- [ ] Add context switch cost benchmarks under realistic mixed workloads.
- [ ] Add per-release performance report included in CI bundle.

### Build, Toolchain, and Reproducibility
- [ ] Add complete build provenance manifest (toolchain, env vars, git state, hashes).
- [ ] Add hermetic build mode for CI with locked external dependency graph.
- [ ] Add source date epoch and deterministic archive ordering checks.
- [x] Add host tool minimum-version checks in host-tool-verify command.
- [ ] Add cross-host reproducibility compare command (Windows/Linux outputs).
- [ ] Add build cache correctness verifier to detect stale artifact reuse.

### Release Engineering and Operations
- [ ] Add pre-release canary track with telemetry-based promote/rollback policy.
- [x] Add automatic release notes generator from gate outputs and change metadata.
- [ ] Add incident simulation drills (boot failure, data corruption, IRQ storm).
- [ ] Add operational playbooks for top 10 failure modes with command snippets.
- [x] Add machine-readable release manifest signed with provenance metadata.
- [ ] Add rollback rehearsal gate that must pass before release promotion.
- [x] Add support diagnostics command for field issue collection.

### Developer Experience and Documentation
- [ ] Add subsystem ownership map and escalation contacts.
- [ ] Add "new contributor" kernel debugging cookbook (QEMU + symbols + traces).
- [ ] Add coding standard for unsafe patterns with approved wrappers/examples.
- [ ] Add documentation coverage checks for public/internal subsystem interfaces.
- [ ] Add architecture diagrams and update cadence policy.
- [ ] Add glossary of kernel/runtime/release terminology used in reports.

### XTask Next-Wave Enhancements
- [x] Add `release gate-report --diff <prev>` command comparing two gate snapshots.
- [x] Add `release explain-failure` command that expands remediation into ordered steps.
- [x] Add `release doctor` command that runs host+repo+artifact sanity checks.
- [x] Add `release trend-dashboard` command for historical gate trend summaries.
- [x] Add `linux-abi semantic-matrix` command for consolidated compatibility semantics scoring.
- [x] Add `linux-abi trend-dashboard` command for Linux ABI history/regression visibility.
- [x] Add `release score-normalize` command to normalize metric drifts across hosts.
- [x] Add `release perf-report` command for release-oriented performance engineering scoring.
- [x] Add `release export-junit` command to emit junit from all release gates.
- [x] Add `release freeze-check` command to enforce branch freeze policy.
- [x] Add `release sbom-audit` command for dependency inventory and policy checks.

### Proposed Execution Waves
1. [ ] Wave 1: Close all gate visibility gaps (warning audit depth, diagnostics depth, ci bundle completeness).
2. [ ] Wave 2: Enforce reliability hard gates (trend, policy, reproducibility, rollback rehearsal).
3. [ ] Wave 3: Expand ABI and compatibility confidence with workload-backed semantics tests.
4. [ ] Wave 4: Stabilize long-run reliability via chaos/soak/perf regression budgets.
5. [ ] Wave 5: Operational excellence (playbooks, canary automation, signed manifests, incident drill scorecards).
