# AetherCore Schedulers

[![Module Status](https://img.shields.io/badge/status-active-brightgreen?style=flat-square)](.)
[![Scheduler](https://img.shields.io/badge/strategy-multi-purple?style=flat-square)](.)

The **Scheduler** determines *which* task runs *when*. In AetherCore, this is a compile-time replaceable module. You can compile a **Unikernel** with a dummy scheduler for maximum throughput or a **Server** with a complex fairness scheduler.

---

## 🧩 Supported Algorithms

| Strategy | Description | Complexity | Use Case |
|----------|-------------|------------|----------|
| **Round Robin (RR)** | Simple, fair, time-sliced. | O(1) | General-purpose, Game loops. |
| **FIFO** | First In, First Out. | O(1) | Batch processing, simple embedded. |
| **LIFO** | Last In, First Out. | O(1) | Cache-hot execution (rare). |
| **Weighted RR** | Priority-aware RR. | O(N) | Soft Real-Time mix. |
| **MLFQ** | Multi-Level Feedback Queue. | O(1) | **Desktop** Interactive + Background. |
| **CFS** | Completely Fair Scheduler. | O(log N) | **Server** High-CPU density. |
| **EDF** | Earliest Deadline First. | O(log N) | **Hard Real-Time** (Deadlines). |

### 🚀 Highlights

1.  **MLFQ (Multi-Level Feedback Queue):**
    *   **Telemetry Required:** Adapts based on CPU vs I/O bursts.
    *   **Anti-Starvation:** Periodic boosting ensures low-priority tasks run eventually.

2.  **EDF (Earliest Deadline First):**
    *   Designed for strict real-time guarantees (e.g., flight control, audio processing).
    *   Requires tasks to declare their computation time and deadline.

### Priority Contract

- Global task priority range is `0..=255`.
- `0` means highest priority, `255` means lowest priority.
- CFS and Lottery both follow this contract:
  - CFS: lower priority value maps to higher weight.
  - Lottery: lower priority value maps to more tickets.
- Boot self-tests enforce this contract (`kernel::scheduler_contract`), so policy drift fails fast during startup.
- Formal policy contract for library-facing service classes is documented in `docs/scheduler_policy_contract.md`.

### Deterministic Replay (Lottery)

- `lottery::deterministic_replay_trace(seed, tasks, picks)` API can produce stable pick traces.
- Same seed + same task list => same trace.
- Different seeds => different traces (used for regression drift detection).

---

## 🛠️ Configuration

Select your desired scheduler in `Cargo.toml` under `[package.metadata.aethercore.config]`.
`build.rs` generates compile-time constants from this metadata, and runtime overrides can be
applied through `modules::schedulers::config`.

```toml
[package.metadata.aethercore.config.scheduler]
strategy = "CFS"              # Options: "RoundRobin", "CFS", "EDF", "FIFO", "Cooperative", "Lottery"
priority_levels = 64

[package.metadata.aethercore.config."scheduler.round_robin"]
max_tasks = 1024
default_slice_ns = 4_000_000

[package.metadata.aethercore.config."scheduler.cfs"]
min_granularity_ns = 1_000_000 # Minimum run time to avoid thrashing
latency_target_ns = 6_000_000  # Target latency period

[package.metadata.aethercore.config."scheduler.lottery"]
initial_seed = 0xCAFEBABE
tickets_per_priority_level = 10
min_tickets_per_task = 1
replay_trace_capacity = 128
lcg_multiplier = 6364136223846793005
lcg_increment = 1
```

Runtime override facade (no rebuild):
- `schedulers::set_scheduler_runtime_config(...)`
- `schedulers::scheduler_runtime_config()`
- includes CFS/MLFQ/EDF/RT and (when enabled) Lottery knobs in one snapshot.
- granular APIs remain available for per-algorithm updates.

---

## 🚧 Roadmap

### Phase 1: Robustness (Q2 2026)
- [x] **Basic Schedulers:** RR, FIFO, LIFO implemented.
- [x] **CFS (Completely Fair Scheduler):** Implemented with Red-Black Tree and Weighted Fair Queuing.
- [x] **MLFQ Tuning:** Queue thresholds and boosting interval made configurable.
- [x] **EDF Deadlines:** Deadline metadata and enforcement hooks integrated.
- [x] **EDF Runtime Telemetry:** Deadline misses/reschedule hints/window resets/throttle events exported for runtime observability.

### Phase 2: SMP (Symmetric Multi-Processing)
- [x] **Per-Core Hook:** `timer_tick_handler` drives scheduler on each core.
- [x] **Per-CPU Runqueues:** True migration-free local queues for scalability > 4 cores. (per-CPU scheduler+runqueue instances are wired through `CpuLocal`; locality/fairness tuning remains incremental)
- [x] **Work Stealing:** A busy CPU steals tasks from an idle CPU to balance load.
- [x] **Gang Scheduling:** Run related threads simultaneously (High-Performance Compute). (baseline gang group API integrated: create/assign/pick + telemetry)

### Phase 3: Advanced
- [x] **Energy-Aware:** Scheduler places tasks on efficient cores (Big.LITTLE) to save power. (baseline efficiency-score CPU picker + telemetry integrated)
- [x] **Real-Time Group Scheduling:** Basic per-group reservation window and utilization cap.
- [x] **RT Guard Feedback Loop:** Kernel starvation guard consumes EDF deadline-pressure signals to reduce forced-reschedule latency under miss bursts.

---

## 🔍 System Analysis Report (Feb 2026)

### 1. Strengths
*   **Algorithmic Variety:** Implements standard (RR) and advanced (CFS) algorithms.
*   **CFS Implementation:** Correctly uses Red-Black Trees (`BTreeMap`) for O(log N) task selection and supports nice values (weights).
*   **SMP Architecture:** Designed with Per-CPU runqueues from day one (via `CpuLocal`), avoiding global lock contention.

### 2. Critical Limitations (SMP)
*   **Per-CPU Runqueue Maturity:** Work stealing and periodic rebalance exist, but queue-local fairness and gang scheduling are still pending.
*   **Migration Policy Depth:** Affinity-aware migration exists, but advanced NUMA/energy-aware heuristics are not yet implemented.

### 3. Production Readiness: **Prototype**
*   **Single Core:** High. Stable algorithms.
*   **Multi Core:** Low. Functional but inefficient due to lack of balancing.

---

## 🔎 Independent Audit Findings (2026-02-19)

### Critical

- **Scheduler maturity differs by algorithm and core-count path:** single-core behavior is stronger than SMP fairness/migration quality under load.

### High

- **MLFQ pick path uses `pop_front().unwrap()` after `front()` check;** functionally safe in current single-threaded queue mutation context, but should be hardened to avoid panic propagation if invariants evolve.
- **Per-CPU runqueue strategy is partially realized** (work stealing/rebalance hooks exist) yet full locality-aware fairness is still incomplete.

### Performance

- **Timer-driven rescheduling is robust**, but cooperative yield and immediate user-driven fairness depend on preemption/tick cadence and guard heuristics.

### Audit Remediation Progress

- [x] MLFQ queue rotation path simplified to avoid `unwrap` and reduce panic-surface in future invariant drift.
- [x] Lottery scheduler now has baseline behavior tests (single-task determinism and valid-id selection under multi-task picks).
- [x] Lottery scheduler now exports runtime telemetry (`add/remove/picks/empty/fallback_first`) for production observability.
- [x] Scheduler test suite expanded with lottery remove-task and weighted-selection behavior checks.
- [x] Expanded SMP-focused fairness telemetry with rebalance-imbalance histogram bins (`lt2/2-3/4-7/8-15/ge16`) in runtime diagnostics.
- [x] Lottery RNG multiplier/increment magic values are now config-driven (`Cargo.toml` + runtime override surface).

### Execution Board (2026)

#### Phase A: Algorithmic Hardening
- [x] Panic-surface reduction in queue rotation paths.
- [x] Expanded lottery and fairness regression tests.
- [x] Add deterministic replay harness for scheduler decisions under fixed seeds.

#### Phase B: SMP Scaling
- [x] Strengthen per-CPU migration heuristics with locality bias budgets.
- [x] Add scheduler pressure classes consumable by library services.
- [x] Add queue pressure percentile telemetry beyond histogram bins.

#### Phase C: Policy Contracts
- [x] Publish formal scheduler policy contract for library-facing service classes.
- [x] Add runtime policy handoff hooks for network/storage service loops.
