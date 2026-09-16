# AetherCore Inter-Process Communication (IPC)

[![Module Status](https://img.shields.io/badge/status-proto-orange?style=flat-square)](.)
[![Performance](https://img.shields.io/badge/ipc-zero__copy-brightgreen?style=flat-square)](.)

Communication within AetherCore is designed to be **Zero-Copy** whenever possible. The IPC subsystem provides the primitives for tasks to synchronize and exchange data safely, whether they are in the same address space (Unikernel) or separated (Process isolation).

---

## 🏗️ Supported Paradigms

| Method | Type | Description | Best For |
|--------|------|-------------|----------|
| **Message Passing** | Sync | Port-based synchronous messaging. | **Microkernel Safety**, decoupled services. |
| **Shared Memory** | Zero-Copy | Remapping physical pages into two address spaces. | **High-Throughout** (Video, Graphics). |
| **Start/Stop Signals** | Async | 64-bit bitmap of notifications. | **HPC**, Synchronization primitives (Wait/Notify). |
| **Ring Buffer** | Zero-Copy | Lock-free circular buffer (Single Producer). | **Networking**, IO streams. |

### 🚀 Highlights

1.  **Zero-Copy Ring Buffers:**
    *   Designed for high-performance I/O (similar to `io_uring`).
    *   Eliminates the need for multiple copies between user/kernel/hardware boundaries.

2.  **Capability-Based Security:**
    *   A task can only send to a "Channel" if it holds the correct Capability Handle.
    *   Prevents unauthorized inter-process communication.

---

## 🛠️ Configuration

Select your desired IPC mechanisms in `hyper_config.toml`. You can enable multiple.

```toml
[ipc]
enable_shared_memory = true
enable_message_passing = true
enable_signals = true
buffer_size_kb = 64
```

---

## 🚧 Roadmap

### Phase 1: Core Primitives (Q2 2026)
- [x] **Shared Memory:** Basic remap logic implemented.
- [x] **Ring Buffer:** Lock-free, single-producer single-consumer circular queue with framing telemetry guards.
- [x] **Futex (Fast Userspace Mutex):** Baseline wait/wake key queues, wake-control IPC bridge, and telemetry counters integrated.

### Phase 2: Advanced IPC
- [x] **Binder-style IPC:** Object-oriented RPC with reference counting (Android style). (baseline object table + refcount acquire/release + transact reply path + telemetry integrated)
- [x] **Unix Domain Sockets:** Standard byte-stream API for POSIX compatibility. (baseline in-memory path-bound byte-stream queue API: `bind/send/recv` + telemetry integrated)
- [x] **D-Bus Message Bus:** Publish/Subscribe model for system events. (baseline topic subscribe/publish/consume queue API + telemetry integrated)

---

## 🔎 Independent Audit Findings (2026-02-19)

### Critical

- **Ring buffer correctness assumptions are SPSC-only and implicit**; API contracts should enforce producer/consumer ownership at type level to prevent accidental misuse.

### High

- **Message framing silently truncates on small receive buffers** (consumer reads partial payload and drops remainder by advancing full frame), which is valid by design but should be documented explicitly as lossy-read semantics.
- **Capability model is documented but not complete across all IPC paths**, leaving policy consistency gaps.

### Performance

- **Current IPC primitives emphasize low overhead**, but backpressure and fairness behavior under sustained high-rate producers need systematic stress testing.

### Audit Remediation Progress

- [x] Removed orphan placeholder IPC module source (`other.rs`) to keep active module surface explicit.
- [x] Added stress tests for ring-buffer wrap-boundary and partial-consumer scenarios.
- [x] Ring-buffer telemetry now tracks full-buffer drops, oversize drops, truncated receives, incomplete-frame observations, and occupancy guard events.
- [x] Message-passing path now enforces bounded message size/depth with oversize/backpressure counters and truncation telemetry.
- [x] Zero-copy path now enforces max payload guard and exports small-buffer/oversize telemetry.
- [x] Signal-only path now exports send/receive hit counters with regression tests.
- [x] Added futex baseline module with key-based wait/wake accounting, wake-event backpressure guard, control-word validation, and regression tests.
- [x] Exposed futex wait/wake through syscall ABI baseline with user-pointer validation and per-syscall telemetry counters.

### Execution Board (2026)

#### Phase A: Contract Hardening
    - [x] Ring-buffer stress and truncation telemetry coverage.
    - [x] Futex baseline with control validation and counters.
    - [x] Type-level ownership contract for SPSC ring producer/consumer endpoints.

#### Phase B: Throughput & Fairness
- [ ] Add producer pressure classes and receiver fairness controls.
- [ ] Add percentile telemetry for queue occupancy and wake latency.
- [ ] Add bounded-priority IPC channels for real-time service classes.

#### Phase C: Policy Integration
- [ ] Unify capability coverage across all IPC paradigms.
- [ ] Publish deployment profile templates (latency-first vs throughput-first).
- [ ] Add cross-subsystem IPC/VFS/network integration scenarios.
