# AetherCore Interrupt Dispatchers

[![Module Status](https://img.shields.io/badge/status-active-yellow?style=flat-square)](.)
[![Latency](https://img.shields.io/badge/latency-low-brightgreen?style=flat-square)](.)

The **Dispatcher** handles the transition from hardware interrupts/exceptions to software handlers. In an Exokernel, the goal is to **"protect but not hide"** the interrupt, allowing applications to register their own handlers directly if permitted.

---

## 🏗️ Architecture

The Dispatcher sits between the **HAL (IDT)** and the **Subsystem Logic**.

```
Hardware IRQ -> CPU -> IDT (HAL) -> Dispatcher (Module) -> Handler (OS/App)
```

Depending on your use case (Hard Real-Time vs Multi-Tenant Server), you choose a dispatch strategy at compile time.

---

## 🧩 Dispatch Strategies

| Strategy | Description | Best For | Latency | Overhead |
|----------|-------------|----------|---------|----------|
| **Direct** | Calls `fn handler()` immediately. | **Hard Real-Time**, simple embedded. | Ultra-Low | Zero (Static Function Call). |
| **Buffered** | Queues IRQ to Per-CPU Buffer. | High-Throughput Networking. | Medium | Buffer management. |
| **Managed** | Filters/Checks permissions. | **Multi-Tenant Server**, secure environments. | High | Permission checks (O(logN)). |
| **Vectored** | Dynamic table of handlers. | General Purpose Desktop, Library OS. | Low-Medium | Indirect Call (O(1)). |

### Usage Scenarios

1.  **Unikernel / Embedded:** Use `Direct` dispatch. No overhead, predictable latency.
2.  **Network Appliance:** Use `Buffered` dispatch. Process packets in batches (NAPI style) to improve instruction cache locality.
3.  **Secure Workstation:** Use `Managed` dispatch to ensure untrusted drivers cannot crash the kernel.

---

## 🛠️ Configuration

Select your desired strategy in `hyper_config.toml`. The `Vectored` strategy is recommended for general-purpose OSs as it allows dynamic handler registration.

```toml
[dispatcher]
strategy = "Vectored"         # Options: "DirectForwarding", "Buffered", "Vectored", "Managed"
buffer_size = 256             # For buffered strategies
vector_table_align = 128      # Alignment for IDT/IVT
```

---

## 🚧 Roadmap

### Phase 1: Robustness (Q2 2026)
- [x] **Vectored Dispatch:** Dynamic registration of handlers at runtime.
- [x] **APIC Integration:** Support for modern x86 interrupt controllers.
- [x] **NMI Watchdog:** Hard-stall panic path integrated (NMI-watchdog-equivalent, ~5s threshold based on runtime tick policy) with runtime telemetry.
- [x] **Interrupt Storm Protection:** Window-based per-IRQ throttling baseline with runtime telemetry (`throttled`, `window_resets`) integrated.

### Phase 2: User-Space Integration
- [x] **User-Mode Callbacks (Upcalls):** Baseline registry/resolve skeleton for IRQ→user callback targets with ownership checks and telemetry.
- [x] **Virtual Interrupts:** Baseline virtual IRQ injection syscall path for user processes with ownership checks and queued delivery telemetry.

### Refactor Direction
- Keep handler context typed and minimal so dispatcher logic does not depend on global mutable state.
- Prefer a small number of shared dispatch helpers over repeated per-strategy branches.
- Push policy decisions to caller-owned contracts and keep the dispatch layer focused on routing, validation, and telemetry.
- Preserve low-latency paths by making the default route direct and moving optional checks behind explicit strategy boundaries.

---

## 🔍 System Analysis Report (Feb 2026)

### 1. Strengths
*   **Dynamic Registration:** Drivers can plug into the interrupt system at runtime (`register_handler`), enabling hot-plugging.
*   **Shared IRQ Support:** Multiple devices can share the same interrupt line (e.g., PCI IRQ A), and the dispatcher will call all registered handlers sequentially.

### 2. Current Limitations
*   **Legacy Routing:** Primarily designed for fixed IRQ/INTx routing. MSI/MSI-X (Message Signaled Interrupts) support is missing, which is critical for high-performance PCIe devices (NVMe, 10GbE).
*   **Linear Scan:** Shared IRQ handling is O(N). If many devices share IRQ 11, latency increases.
*   **Safety:** Handlers are raw function pointers. No context (`void* arg`) is passed, forcing drivers to use `static mut` or global locks to find their device instance. This is anti-pattern for reentrant drivers.

### 3. Production Readiness: **Stable for Legacy**
*   **Robustness:** Good for standard devices (Serial, Timer, IDE).
*   **Scalability:** Poor for high-end servers without MSI-X.

---

## 🔎 Independent Audit Findings (2026-02-19)

### Critical

- **Dispatch safety model is still pointer-centric:** handler registration and invocation contracts need stronger typed context ownership to avoid global mutable driver state coupling.

### High

- **MSI/MSI-X gap remains a major throughput limiter** for modern NVMe/NIC workloads.
- **Shared IRQ fan-out remains linear scan**, increasing tail latency under multi-device sharing.

### Performance

- **No explicit interrupt backpressure policy contract** documented for storm and high-rate source scenarios beyond baseline throttling hooks.

### Audit Remediation Progress

- [x] Removed orphan placeholder dispatcher source (`other.rs`) to avoid drift from active dispatch module surface.
- [x] Added shared-IRQ fan-out telemetry baseline (`dispatch/register/default/invocation/max_fanout/storm_hints`) and runtime observability logs.
- [x] Added baseline storm throttling policy for noisy IRQ lines with dispatch-window reset accounting and regression tests.
- [x] Added upcall registry baseline (register/resolve/unregister ownership semantics) with runtime telemetry hooks and regression tests.
- [x] Wired vectored dispatch to consult global upcall targets and mark delivery hints for future user-return path integration.
- [x] Added syscall control-plane bridge for upcall register/unregister/query with process ownership checks and user-pointer-safe query serialization.
- [x] Added upcall pending-delivery queue and consume syscall, plus virtual IRQ inject syscall baseline for user-process callback testing.
- [x] Added hard watchdog panic threshold integration (`nmi_watchdog_emulation`) for prolonged stalled CPU detection beyond soft watchdog threshold.

### Execution Board (2026)

#### Phase A: Safety & Observability
- [x] Storm throttling baseline with telemetry and tests.
- [x] Upcall registration/consume baseline integrated.
- [x] Typed handler context contracts to reduce global mutable state dependence.

#### Phase B: Throughput Scaling
- [ ] MSI/MSI-X aware dispatch path integration for modern PCIe devices.
- [x] Shared IRQ fan-out optimization beyond linear scans.
- [ ] Per-IRQ scheduling hints for downstream service loops.

#### Phase C: User-Space Dispatch Services
- [ ] Mature virtual IRQ and upcall QoS classes.
- [ ] Add secure callback capability delegation model.
- [ ] Publish dispatcher policy playbooks by workload profile.
