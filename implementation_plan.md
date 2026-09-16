# AetherXOS RTOS Transformation & Certification Plan (VxWorks Equivalent)

This document outlines the detailed architectural path required to transition AetherXOS to support a deterministic, hard real-time operating system profile, fully prepared for safety-critical certifications (DO-178C DAL A and ISO 26262 ASIL D), equivalent to industry standards like VxWorks.

## Architectural Philosophy: Configuration-Driven RTOS

As discussed, converting the entire OS to strict RTOS requirements permanently would damage the high-bandwidth/low-latency capabilities of the standard Exokernel. Therefore, **Determinism and Certification are treated as a Configuration Profile**. 

Using AetherXOS's declarative config system (`build_cfg`), we will introduce a strict `rtos_certified` profile. When this profile is activated at compile time:
* Dynamic memory allocation (`kmalloc`) in the fast path is mathematically blocked.
* The scheduler switches into a mathematically proven O(1) mode.
* Panic handlers transition to fail-safe halt/reset mechanisms.

---

## Dual-API Strategy: POSIX-RT + Native Exokernel

> [!TIP]
> **API Compatibility (Both POSIX & LibraryOS)**
> To support legacy aerospace/automotive software while maintaining modern capabilities, we will implement a dual-API architecture. The foundational kernel constructs will be deterministic and certified. On top of this substrate, two interfaces will be exposed:
> 1.  **POSIX Real-Time Extensions:** We will implement standard `pthread_setschedparam`, POSIX message queues (`mqueue.h`), and POSIX timers. This allows existing VxWorks/Linux-RT code to run unchanged.
> 2.  **Native LibraryOS API:** For new applications aiming for minimum possible overhead without the POSIX abstraction tax, the direct memory and scheduling models of AetherXOS will be accessible under the same hard real-time guarantees.

---

## Proposed Changes (Phase 1: Foundation)

### 1. The RTOS Configuration Profile

**Goal:** Allow the kernel to be conditionally compiled for certification.
- **Changes:**
  - Create a new runtime configuration schema in `build_cfg` that enables `#![cfg(feature = "rtos_strict")]`.
  - When enabled, the kernel will compile with specific static assertions to ensure MISRA-Rust/Ferrocene compatibility.

#### [NEW] `config/profiles/rtos.toml`
#### [MODIFY] `build_cfg/src/schema.rs`

### 2. Conditionally Bounded Memory Management

**Goal:** Zero-Heap in Fast Path, but only for the RTOS profile.
- **Changes:**
  - Standard builds will use high-performance dynamic memory.
  - RTOS builds will disable standard `alloc::` calls after initialization (Phase 0).
  - Introduce **Slab Allocators** and **Static Object Pools** triggered specifically under `cfg(feature="rtos_strict")`.

#### [NEW] `kernel/src/kernel/memory/rt_pools.rs`
#### [MODIFY] `kernel/src/kernel/memory/mod.rs`

### 3. Hardened Real-Time Scheduler

**Goal:** Temporally isolated execution.
- **Changes:**
  - Enhance `kernel/src/kernel/rt_preemption.rs` to enforce hard CPU time-slice limits.
  - Implement task affinity mapping so critical DO-178C tasks cannot be interrupted by lower-priority logging or management tasks.

#### [MODIFY] `kernel/src/kernel/rt_preemption.rs`
#### [MODIFY] `kernel/src/kernel/task.rs`

### 4. Bounded Interrupt Processing

**Goal:** Deterministic ISR Latency.
- **Changes:**
  - Standardize Top-Half (Immediate) / Bottom-Half (Deferred) interrupt splitting across the HAL.
  - Implement un-interruptible execution tracking for `interrupt_guard.rs`.

#### [MODIFY] `kernel/src/kernel/interrupt_guard.rs`

### 5. POSIX RT Compatibility Layer (New)

**Goal:** Provide industry-standard VxWorks/Linux-RT interfaces.
- **Changes:**
  - Implement a POSIX RT facade on top of the native scheduler.
  - Add thread priority attribute parsing, POSIX semaphores, and strict real-time timers bridging into AetherXOS native primitives.

#### [NEW] `kernel/src/kernel/api/posix_rt.rs`
#### [MODIFY] `kernel/src/kernel/api/mod.rs`

---

## Verification Plan

### Automated Tests
- **RTOS Profile CI Tests:** Add a new `xtask` command (e.g., `xtask test --profile rtos`) that specifically runs the test suite compiled against the strict allocation bounds and O(1) scheduler logic.
- **Latency Measurement:** Kani model checking for `pi_mutex.rs` to prove no priority inversion occurs, plus integrated QEMU runtime tests verifying interrupt-to-work latency delta.
