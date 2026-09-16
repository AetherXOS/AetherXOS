# AetherCore HAL (Hardware Abstraction Layer)

[![Module Status](https://img.shields.io/badge/status-stable-green?style=flat-square)](.)
[![Arch](https://img.shields.io/badge/arch-x86__64-orange?style=flat-square)](x86_64)

The **Hardware Abstraction Layer (HAL)** is the foundation of AetherCore. It isolates the generic kernel logic from architecture-specific details, allowing the OS to run on `x86_64`, `AArch64`, and `RISC-V` with minimal changes to the core code.

---

## 🏗️ Architecture

The HAL defines a set of **Traits** in `src/interfaces` that every architecture must implement.

| Trait | Description |
|-------|-------------|
| `ContextSwitch` | Saving and restoring CPU registers (`RIP`, `RSP`, general-purpose registers). |
| `InterruptController` | Managing IRQs (APIC on x86, GIC on ARM). |
| `Mmu` | Page table manipulation (CR3 on x86, TTBR on ARM). |

---

## 🧩 Supported Architectures

### 1. `x86_64` (Intel/AMD)
*   **Status:** ✅ Stable
*   **Features:**
    *   **GDT/TSS:** Per-CPU setup for SMP safety.
    *   **IDT:** Vectored interrupt handling with APIC EOI.
    *   **APIC:** Modern Local APIC driver (replaces legacy 8259 PIC).
    *   **SMP:** Multicore boot support via Limine protocol.
    *   **PCI:** Bus enumeration and device discovery.
    *   **Syscalls:** Fast `syscall`/`sysret` interface.

### 2. `AArch64` (ARMv8)
*   **Status:** 🚧 In Implementation
*   **Target:** Raspberry Pi 4, QEMU `virt` machine.

### Architecture Parity Matrix

| Feature / Trait | x86_64 | AArch64 | RISC-V |
| --- | --- | --- | --- |
| Context Switch | 🟢 Done | 🟢 Done | 🔴 N/A |
| Exception Vectors | 🟢 Done | 🟢 Done | 🔴 N/A |
| Interrupt Controller | 🟢 Done | 🟡 GICv2 | 🔴 N/A |
| MMU/Paging | 🟢 Done | 🟡 Basic | 🔴 N/A |
| Serial Driver | 🟢 16550 | 🟢 PL011 | 🔴 N/A |
| Timer | 🟢 APIC | 🟢 Generic | 🔴 N/A |
| Virtualization | 🟢 VMX/SVM | 🟢 EL2 cap | 🔴 N/A |

---

## 🚧 Roadmap

### Phase 1: Symmetric Multi-Processing (SMP) (Q2 2026)
- [x] **APIC Initialization:** Enable Local APIC and Timer on all cores.
- [x] **SMP Boot:** Wake up Application Processors (APs) via Limine.
- [x] **Per-CPU Storage:** `GS` base switching for `CpuLocal` data.
- [x] **Locking:** `IrqSafeMutex` for critical HAL sections.

### Phase 2: Hardware Discovery
- [x] **PCI Enumeration:** Scan bus for devices (Network, Disk, GPU).
- [x] **ACPI:** Parse `MADT` tables for advanced IOAPIC topology.
- [x] **Device Tree (DTB):** For ARM/RISC-V hardware discovery. (bootloader DTB discovery hook integrated)

### Phase 3: Advanced Features
- [x] **IOMMU:** VT-d / AMD-Vi ACPI table discovery (DMAR/IVRS) + hardware backend selection + DMA isolation bookkeeping + domain/device attach + VT-d root/context bootstrap + per-domain second-level context pointer wiring + hierarchical root→leaf second-level entry population + AMD-Vi command ring with global/domain/device invalidate paths + ring-full guard/fallback + runtime flush invalidation hookup + per-scope invalidation telemetry.
- [x] **Virtualization:** VMX/SVM launch-readiness baseline integrated. (CPU capability probing + VMX/SVM enable + VMXON entry path + runtime VM launch readiness telemetry/guard + VMCS/VMCB preparation baseline + launch-context lifecycle API integrated; full nested guest entry/exit policy remains incremental)

### Refactor Direction
- Keep architecture-specific code inside `hal/` and expose only typed, architecture-neutral traits to the rest of the kernel.
- Prefer small adapters at the boundary over repeated `#[cfg]` branches in deeper logic.
- Move shared conversion, validation, and telemetry logic into one helper per subsystem.
- Treat driver and dispatcher contracts as stable interfaces; policy should depend on contracts, not device internals.

---

## 🔍 System Analysis Report (Feb 2026)

### 1. Strengths
*   **Modular Design:** Clean separation between `gdt`, `idt`, `apic`, and `pci`.
*   **SMP Support:** successfully boots APs using Limine protocol and initializes per-CPU structures (`CpuLocal` via `GS` register).
*   **Modern Interrupts:** Abandoned legacy PIC for APIC, enabling proper multicore interrupt routing.
*   **PCI Scanning:** Robust enumeration of PCI bus, including multi-function devices and BAR reading.

### 2. Current Limitations
*   **ACPI depth:** `MADT` discovery/parsing is available, but full IOAPIC programming policy integration is still incremental.
*   **IOMMU depth:** VT-d hierarchical root→leaf second-level entry management is integrated for mapped pages and unmap cleanup, and AMD-Vi command ring/polling with global/domain/device invalidate plus ring-full guard/fallback is integrated; remaining work is broader hardware-specific command/feature coverage.
*   **Virtualization depth:** CPU capability detection and enable path are integrated with VM launch readiness guard/telemetry, VMCS/VMCB preparation baseline, and launch-context lifecycle API (`initialize/reset/teardown`) with lifecycle telemetry; remaining work is full guest entry/exit control flow.
*   **x86 Only:** ARM/RISC-V support is currently at early bring-up level.

### 3. Production Readiness: **Beta**
*   **Stability:** High. The core boot path is stable on QEMU and standard hardware.
*   **Security:** Medium. `Ring 3` syscalls are implemented but lack full validation of user pointers (`access_ok` checks).
*   **Scalability:** Tested up to 64 cores. Scalability is currently limited by the `Global Allocator` lock contention, not the HAL itself.

---

## 🧪 Testing

The HAL is tested via generic integration tests and QEMU-specific integration scripts.

```bash
# Test x86_64 HAL
cargo test --target x86_64-unknown-none --lib src/hal
```

---

## 🔎 Independent Audit Findings (2026-02-19)

### Critical

- **Syscall boundary checks are incomplete:** user pointers are range-checked but not fully validated for mapped/readable/writable accessibility before raw slice construction.
- **Interrupt/virtualization globals rely on mutable static state:** expected in low-level code, but `static mut` regions (e.g., virtualization state, dispatcher pointer) need stricter safe wrappers and single-writer guarantees.

### High

- **AArch64 path is still mostly scaffolding** while docs position multi-arch parity; this can create portability assumptions that are not yet true.
- **ACPI/IOMMU parsing uses raw memory reads extensively**; current approach works for controlled boot assumptions but needs stronger bounds/structure validation hardening.

### Performance

- **Spin-based waits remain in several hardware paths** (serial, invalidation wait loops); bounded wait + fallback policies should be standardized.

### Audit Remediation Progress

- [x] Syscall user-buffer validation strengthened from pure range-check to mapping-aware permission check (`PRESENT/USER`, and `WRITABLE` for write paths).
- [x] Serial TX path now uses bounded spin-wait with timeout/drop counters to avoid unbounded busy-wait lockup under stuck UART conditions.
- [x] HAL runtime logs now include serial telemetry (`tx_bytes`, `drops`, `spin_loops`, `timeouts`) for operational diagnostics.
- [x] Add explicit fault-safe copyin/copyout abstraction that can report fine-grained page fault cause (not just invalid-arg).

### Core-Library Boundary Support Track (2026 H1)

- [x] HAL-backed serial and interrupt telemetry now feed higher-level runtime observability, enabling library policy loops to react safely.
- [x] IOMMU/virtualization readiness metrics are exposed to runtime policy layers without embedding policy in HAL.
- [x] Standardize bounded wait policy metadata across HAL drivers for direct consumption by library-side adaptive controllers.
- [ ] Introduce architecture-neutral NIC dataplane trait contracts to reduce coupling between core bridge and architecture-specific drivers.

### Execution Board (2026)

#### Phase A: Safety Foundations
- [x] Mapping-aware user pointer validation support completed at syscall boundaries.
- [x] Bounded serial wait semantics and telemetry integrated.
- [x] Strengthen raw ACPI/PCI parsing bounds checks with structured validation outcomes.

#### Phase B: Multi-Arch Progress
- [ ] Advance AArch64 from scaffolding to runnable interrupt/mmu baseline.
- [ ] Add architecture parity matrix for HAL trait coverage.
- [ ] Add cross-arch smoke suite for boot + interrupt + timer core paths.

#### Phase C: Dataplane Contracts
- [ ] Add architecture-neutral NIC queue contract for core bridge integration.
- [x] Standardize driver wait/timeout policy descriptors across HAL-backed devices.
- [ ] Add hardware capability export model consumed by library policy engines.

### Delivery Waves (HAL Track)

#### Wave H1: Validation Hardening
- [x] Strengthen ACPI/PCI parser bounds and structured error reporting.
- [x] Add parser validation telemetry for malformed descriptor paths.

#### Wave H2: Multi-Arch Progress
- [x] Advance AArch64 interrupt/mmu/timer baseline to runnable level.
- [x] Add architecture parity checklist for HAL trait coverage.

#### Wave H3: Dataplane Contracts
- [ ] Finalize architecture-neutral NIC queue traits.
- [x] Standardize wait/timeout descriptor exports for driver policy engines.

### Acceptance Criteria (HAL)
- HAL remains policy-agnostic while exporting complete capability metadata.
- All bounded-wait hardware paths expose timeout/drop telemetry.
- Cross-architecture boot + interrupt smoke suite passes consistently.
