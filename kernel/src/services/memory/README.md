# AetherCore Memory Allocators

[![Module Status](https://img.shields.io/badge/status-stable-green?style=flat-square)](.)
[![Allocation](https://img.shields.io/badge/strategy-multi-purple?style=flat-square)](.)

This module provides pluggable Memory Allocation strategies for both Physical Frames (PMM) and Virtual Kernel Heap (KMM). As an **Exokernel**, AetherCore allows applications to choose *how* memory is managed or even implement their own allocators in userspace.

---

## 🧩 Supported Strategies

| Strategy | Type | Description | Best For |
|----------|------|-------------|----------|
| **Bitmap** | Physical | Tracks frames with a bit array. | Low memory overhead, predictable fragmentation. |
| **Buddy** | Physical | Splits blocks in powers of 2. | Balancing fragmentation and coalescing speed. |
| **Bump** | Heap | Linear allocation, no `free()`. | Extremely fast, short-lived tasks (Unikernels). |
| **Linked List** | Heap | Traditional `malloc`/`free`. | General-purpose dynamic workloads. |
| **Slab** | Heap | Caches for fixed-size objects. | **Kernel Objects** (Task structs, Inodes), specific object pools. |

### 🚀 Key Features

1.  **Per-CPU Caches (Magazines):**
    *   Thread-local allocation buffers to eliminate lock contention on multicore systems.
    *   Similar to `jemalloc` or `tcmalloc`.

2.  **Huge Page Support:**
    *   Allocator is aware of 2MB (Large) and 1GB (Huge) pages to reduce TLB misses.
    *   Critical for database, scientific, and virtualization workloads.

---

## 🛠️ Configuration

Select your desired allocator in `hyper_config.toml` to compile *only* that code.

```toml
[memory]
allocator = "Slab"             # Options: "Bump", "LinkedListAllocator", "Slab", "Buddy"
paging = true                  # Enable MMU/Paging
heap_size_mb = 32              # Initial Kernel Heap Size
guardian_pages = true          # Stack overflow detection

# Slab tuning
slab_refill_bytes = 16384      # Bytes fetched from fallback allocator per refill
slab_cache_limit = 64          # Per-CPU per-size-class soft max cached blocks
slab_release_batch = 32        # Max blocks released to fallback when cache is over limit
slab_cross_cpu_steal = true    # Allow scavenging free blocks from other CPU magazines
slab_reclaim_profile = "Balanced" # Conservative|Balanced|Aggressive reclaim policy class
slab_pressure_scan_budget = 8   # Max reclaim candidate scans per pressure pass
slab_max_tracked_segments = 1024 # Segment tracking table size for full reclaim

# Advanced policy tuning
compaction_budget_pages = 1024 # Upper bound for compaction passes
oom_kill_threshold = 1         # Minimum OOM score eligible for victim selection
prefer_local_numa = true       # true=local-node bias, false=spread bias
```

---

## 🚧 Roadmap

### Phase 1: Optimization (Q2 2026)
- [x] **Per-CPU Caches (Magazines):** Thread-local allocation buffers implemented in Slab allocator.
- [x] **Bulk Refill:** Amortize lock contention by fetching batches of objects.
- [x] **SLUB Support:** Unqueued slab allocator for better CPU cache locality. (baseline SLUB allocation API + telemetry integrated)
- [x] **Memory Compaction:** Defragmenting physical memory in the background (kcompactd). (baseline compaction pass API + run telemetry integrated)

### Phase 2: Advanced Features
- [x] **NUMA Awareness:** Preferring memory from the local socket to reduce latency. (baseline preferred-node hint API integrated)
- [x] **OOM Killer:** Strategy for reclaiming memory under pressure (kill vs squeeze). (baseline score table + victim selection API integrated)
- [x] **Memory Hot-Plug:** Supporting physical RAM addition at runtime. (baseline add-memory-pages API + capacity telemetry integrated)

---

## 🔍 System Analysis Report (Feb 2026)

### 1. Strengths
*   **Slab Allocator:** Production-ready implementation with strict object size segregation (32B - 4KB) and per-CPU caching. Drastically reduces lock contention.
*   **Configurable Backend:** Config-switchable allocators allow easy debugging (Bump) vs performance (Slab).
*   **Safety:** Memory safety guarantees (no use-after-free in safe code) are upheld by Rust's ownership model wrapper.

### 2. Current Limitations
*   **Fragmentation:** Fallback `LinkedListAllocator` suffers from external fragmentation over long uptimes.
*   **Reclaim Pipeline:** Slab now tracks refill segments and reclaims fully-free segments back to the system allocator.
*   **Fixed Size Heap:** Heap size is fixed at boot (`boot_config.heap_size_mb`). No dynamic growth (brk/sbrk equivalent for kernel heap).

### 3. Production Readiness: **Early Production**
*   **Reliability:** tested with heavy object churn (Slab test loop).
*   **Performance:** High. Allocation path is often lock-free (local cache hit).

---

## 🤝 Contributing

To add a new allocator:
1.  Implement the `FrameAllocator` (Physical) or `GlobalAlloc` (Heap) trait.
2.  Add your module in `src/modules/allocators/`.
3.  Add options in `hyper_config.toml` and `build.rs`.

---

## 🔎 Independent Audit Findings (2026-02-19)

### Critical

- Legacy allocator surface duplication previously existed; duplicate `src/modules/allocator.rs` has been removed and active allocator surface is now `src/modules/allocators/*`.

### Audit Remediation Progress

- [x] Removed obsolete duplicate legacy allocator module (`src/modules/allocator.rs`) to eliminate null-returning placeholder confusion.
- [x] Slab allocator now exports runtime telemetry (`alloc_calls/fast/refill/steal/fallback/refill_failures`) for production observability.
- [x] Slab refill path now avoids panic-prone layout construction (`unwrap`) and uses explicit fail-safe layout derivation.
- [x] Added explicit fail-fast diagnostics for unsupported allocator profile fallback path (`JemallocLite`) with runtime telemetry.

### High

- **Bitmap PMM uses global mutable bitmap state** with no intrinsic synchronization in the allocator itself; SMP safety currently depends on external call discipline.
- **Fallback allocator paths should be explicit fail-fast** in non-debug builds to avoid silent null-allocation behavior in accidental integration paths.

### Performance

- **Bitmap first-fit remains O(n)** and can degrade under sustained pressure.
- **Slab reclaim coverage improved** with tracked segment reclaim, profile classes, and queue/latency percentile telemetry.

### Execution Board (2026)

#### Phase A: Reliability Hardening
- [x] Removed legacy duplicate allocator module surface.
- [x] Added slab observability and panic-surface reduction.
- [x] Add allocator contract tests for extreme fragmentation and exhaustion patterns.

#### Phase B: Pressure Handling
- [x] Complete slab page reclaim pipeline back to system allocator.
- [x] Add adaptive reclaim policy classes by workload profile.
- [x] Add queue-depth and reclaim-latency percentile telemetry.

#### Phase C: Scale & Topology
- [ ] Expand NUMA policy from hints to active balancing heuristics.
- [ ] Add hot-plug rebalance safety checks for long-running systems.
- [ ] Publish allocator profile guide for kernel/service classes.

### Refactor Direction
- Keep allocator interfaces small and explicit so higher layers do not depend on backend-specific internals.
- Consolidate repeated allocation/reclaim patterns into shared helpers instead of duplicating them per profile.
- Make fallback behavior fail-fast and visible rather than silently returning placeholder values.
- Prefer telemetry at the boundary where decisions are made, not inside every call site.
