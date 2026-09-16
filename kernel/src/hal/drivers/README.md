# AetherCore Drivers

This module contains device drivers for various hardware components.

## 🧩 Supported Drivers

| Driver | Hardware | Bus | Status |
|--------|----------|-----|--------|
| `VirtIoNet` | VirtIO Network Card | PCI | 🚧 RX/TX + Control Queue Baseline |
| `Nvme` | NVMe SSD Controller | PCI | 🚧 Probe + Init + Queue-Depth Hooks |
| `Ahci` | SATA/AHCI Controller | PCI | 🚧 Probe + Init Baseline |
| `VirtIoBlock` | VirtIO Block Device | PCI | 🚧 Probe + Init Baseline |
| `StorageManager` | Multi-Driver Block Orchestrator | N/A | ✅ Active Baseline |
| `Serial` | 16550 UART | ISA/IO | ✅ Working |
| `LocalAPIC` | Intel Local APIC | MMIO | ✅ Working |

## 🛠️ Usage

Drivers are typically initialized during the kernel boot sequence in `main.rs`.

```rust
use aethercore::modules::drivers::VirtIoNet;

if let Some(mut net) = VirtIoNet::probe(&devices) {
    net.init().expect("Failed to init network");
}
```

### Network Driver Catalog Flow

Network driver probing/selection is now centralized in `drivers::catalog`.

When adding a new network driver:

1. Implement `PciProbeDriver` + `DriverLifecycle` for the concrete driver type.
2. Extend `ProbedNetworkDriver` in `catalog.rs` with the new concrete driver variant.
3. Add a probe function and `NetworkProbeStep` entry to `NETWORK_PROBE_PLAN` (ordered by priority).
4. Wire dataplane registration in runtime by matching `ProbedNetworkDriver` variant.

This keeps probe order deterministic and avoids scattering runtime `if/else` probe logic.

### Driver Authoring Quickstart

For new PCI drivers, prefer shared probe helpers from `drivers::probe`:

```rust
use aethercore::modules::drivers::{pci_id, probe_first_pci_by_ids, pci_bar0_mmio_base};

let ids = [pci_id(0x8086, 0x1234)];
let dev = probe_first_pci_by_ids(devices, &ids).ok_or("device not found")?;
let mmio = pci_bar0_mmio_base(dev).ok_or("mmio bar0 missing")?;
```

Storage probing is also catalog-based (`StorageManager::probe_plan`), so adding a new storage driver means:

1. Implement `PciProbeDriver + DriverLifecycle + BlockDevice`.
2. Add variant to `ProbedStorageDriver`.
3. Add probe callback entry into `STORAGE_PROBE_PLAN`.
4. Done, manager picks it up automatically.

Storage manager capacity is now dynamic (`Vec`-backed), so adding a new storage driver no longer requires changing fixed array sizes.

Lifecycle boilerplate is centralized through `LifecycleAdapter + impl_lifecycle_adapter!`.
Implement driver-specific `lifecycle_init/lifecycle_service/lifecycle_teardown` methods and wire once with macro.

Runtime network driver ownership is centralized in `drivers::registry` with:

1. `register/unregister` APIs
2. `hotplug_attach/hotplug_detach` APIs
3. runtime snapshot telemetry (`runtime_registry_snapshot`)
4. recent event stream (`runtime_registry_events`, bounded ring)
5. detach-time active-path quiesce + automatic standby failover activation

Runtime driver tuning facade is available via:

- `drivers::driver_network_runtime_config()`
- `drivers::set_driver_network_runtime_config(...)`
- `drivers::driver_storage_runtime_config()`
- `drivers::set_driver_storage_runtime_config(...)`
- `drivers::driver_wait_runtime_config()`
- `drivers::set_driver_wait_runtime_config(...)`

This aligns driver dataplane budgets/ring limits/quarantine/SLO thresholds, poll-profile
low-latency divisors/throughput multipliers, E1000 buffer-size + descriptor-count tuning, and NVMe queue profile/depth overrides with
AHCI/NVMe/E1000 wait-timeout spins with `KernelConfig` runtime overrides from a single API surface.

Network probe preference can be configured through `drivers::policy`:

- `PreferVirtIo` (default)
- `PreferE1000`
- `VirtIoOnly`
- `E1000Only`

SLO remediation profile is also configurable:

- `Conservative`: slower escalation, longer cooldown.
- `Balanced` (default): rebind before failover, moderate cooldown/jitter.
- `Aggressive`: faster failover, shorter cooldown, skips rebind stage.

---

## 🔍 System Analysis Report (Feb 2026)

### 1. Strengths
*   **Modular Probing:** Drivers check `PciDevice` list for Vendor/Device ID matches, decoupling detection from initialization.
*   **Protocol Compliance:** VirtIO driver follows Legacy spec for handshake (ACK -> DRIVER -> FEATURES_OK -> DRIVER_OK).
*   **IO Abstraction:** Uses `x86_64::Port` for safe I/O port access.

### 2. Current Limitations
*   **VirtIO:** Legacy RX/TX virtqueue + control-queue baseline exists, but advanced offload/feature parity (segmentation/checksum/multiqueue depth) is still incremental.
*   **Serial:** Uses `spin_loop` (Polling) for transmission. Blocking the CPU for I/O is unacceptable for high-throughput logging.
*   **Driver Model:** Unified lifecycle trait + standardized state machine exists (`DriverState/DriverErrorKind/DriverStatus`) and is integrated into network + storage probe/init paths; full cross-driver dependency graph and runtime rebind flows remain incremental.

### 3. Production Readiness: **Experimental**
*   **Networking:** Functional baseline with staged dataplane maturity.
*   **Console:** Functional but slow (Polling).

---

## 🔎 Independent Audit Findings (2026-02-19)

### Critical

- **VirtIO-net advanced feature parity is incomplete:** control queue baseline exists, but production parity still needs full offload/multiqueue depth and broader hardware validation.

### High

- **No unified lifecycle trait for all drivers** (probe/init/io/teardown/error state), increasing integration drift and test complexity.
- **Initialization sequencing is tightly runtime-scripted** from kernel runtime paths, reducing isolation and modular testability.

### Performance

- **Serial output remains polling/busy-wait based** and can become a dominant stall source under verbose logging.

### Audit Remediation Progress

- [x] Added driver-agnostic storage orchestration baseline (`StorageManager`) with unified block info and probe integration hooks.
- [x] Network/runtime path now uses facade boundaries (`network::bridge`) reducing direct monolithic coupling.
- [x] Serial path telemetry is exposed for timeout/drop diagnostics under bounded wait behavior.

## 🚧 Roadmap

### Phase 1: Driver Lifecycle Unification (In Progress)
- [x] Introduce unified driver trait (`probe/init/io/teardown/health`) baseline for major network drivers and runtime integration points.
- [x] Add standardized driver health state machine and error taxonomy baseline (`DriverState`, `DriverErrorKind`, `DriverStatus`) for network + storage drivers.
- [x] Add deterministic probe ordering baseline and dependency metadata support for storage probe plan.
- [x] Add auto-recovery budget and cooldown policy to lifecycle state machine (`DriverRecoveryPolicy`) with runtime IO gating.

### Phase 2: Dataplane Maturity
- [x] Complete VirtIO-net RX/TX virtqueue dataplane baseline.
- [x] Add E1000 dataplane parity and throughput telemetry baseline.
- [x] Add NVMe queue-depth tuning profile hooks.

### Phase 3: Production Operations
- [x] Add hot-reload safe driver reset/rebind workflows baseline.
- [x] Add driver-specific SLO dashboards (latency, drop, timeout, reset rates) baseline.
- [x] Add SLO-breach driven automatic policy/failover switching with cooldown guard.
- [x] Add fault-injection coverage for queue saturation/SLO breach detection paths.
- [x] Standardize wait/timeout policy descriptor snapshot across NVMe/AHCI/E1000 for runtime policy engines.
- [ ] Publish deployment guide for VM vs bare-metal driver profiles.
