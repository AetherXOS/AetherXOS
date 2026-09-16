## Onion Architecture: Katmanlı Mimari & İzolasyon

This kernel implements a strict **layered onion architecture** with clear isolation boundaries and zero removal of existing modules/features.

### Architecture Layers (from core to periphery)

```
┌─────────────────────────────────────────────────────┐
│  Applications / Drivers (Feature-gated modules)     │  Top Layer
├─────────────────────────────────────────────────────┤
│  Services (Facade: scheduler, vfs, memory, drivers) │  High-level API
├─────────────────────────────────────────────────────┤
│  HAL Bridge (CoreHal → existing interfaces)         │  Hardware Abstraction
├─────────────────────────────────────────────────────┤
│  HAL Devices (UART, Timer, Interrupt Controller)    │  Type-Safe MMIO
├─────────────────────────────────────────────────────┤
│  Core (Traits, Types, Error, Log, Time facades)     │  Zero-Cost Abstractions
└─────────────────────────────────────────────────────┘
```

### 1. Core Layer (`kernel/src/core/`)

**Purpose:** Architecture-neutral contracts, traits, and common types.

- **`error.rs`**: Re-exports `KernelError` and `KernelResult`.
- **`traits/hardware.rs`**: Hardware abstraction traits (scheduler, I/O).
- **`types.rs`**: Common types like `CapabilityToken`, `MmioBase<const BASE>`.
- **`log.rs`**: Logging facade for AOP macros and diagnostics.
- **`time.rs`**: Cross-platform cycle counter and timing utilities.

**Design Principle:** All code depends on traits, not concrete implementations.

### 2. HAL Layer (`kernel/src/hal/`)

**Purpose:** Hardware abstraction with const-generic, type-safe MMIO access.

- **`bridge.rs`**: Maps core traits to existing HAL implementations (non-destructive).
- **`mmio/`**: 
  - `typed_mmio.rs`: `VolatileCell<T>`, `MappedRegion<const BASE>` for zero-cost MMIO.
- **`devices/`**: Type-state enforced device drivers:
  - `uart.rs`: `Uart<const BASE>` with Write trait impl.
  - `timer.rs`: `Timer<const BASE, STATE>` with type-state for initialization.
  - `interrupts.rs`: `InterruptController<const BASE>` with capability-based access.

**Key Properties:**
- **Const Generics**: `Uart<const BASE: usize>` eliminates runtime base address overhead.
- **Type-State**: `Timer<BASE, Initialized>` prevents use-before-init.
- **Capability Tokens**: `IrqMaskCapability` enforces authorization for IRQ operations.

### 3. Services Layer (`kernel/src/services/`)

**Purpose:** High-level facades that re-export existing modules without renaming.

- **`scheduler.rs`**: Re-exports schedulers, selector, and config—preserves all scheduler types.
- **`vfs.rs`**: Conditional re-export of VFS module (feature: `vfs`).
- **`memory.rs`**: Re-exports allocators, persistent_memory, memory_safety.
- **`drivers.rs`**: Conditional re-export of drivers (feature: `drivers`).

**Design Principle:** No existing functionality is removed; only re-organized for clarity.

### 4. BSP Layer (`kernel/src/bsp/`)

**Purpose:** Board/System configuration and bootstrapping.

- **`macros/mod.rs`**: Declarative macros (e.g., `define_system_board!`) for board-specific setup.
- **`common/`**: Shared boot constants and bootloader contracts.

### 5. AOP Layer (`aop_macros/`)

**Purpose:** Aspect-Oriented Programming macros for cross-cutting concerns.

Attribute macros for logging and tracing without modifying function bodies:

- **`#[log_entry]`**: Entry/exit logging with configurable levels (trace/debug/info/warn/error).
- **`#[irq_handler(priority = N)]`**: IRQ tracking with automatic cycle counting.
- **`#[perf_trace(threshold = N)]`**: Performance warnings for slow operations.

All macros call `crate::core::log::log_event()` for output.

### Safety & Justification

#### Unsafe Blocks: MMIO and Device Access

**Location**: `hal/mmio/typed_mmio.rs`, `hal/devices/uart.rs`, `hal/devices/timer.rs`, `hal/devices/interrupts.rs`

**SAFETY Contract:**
```
// SAFETY: BASE points to valid, readable/writable MMIO region.
// Callers must ensure:
// 1. BASE is properly page-aligned for the target architecture.
// 2. MMIO region persists for the kernel's lifetime.
// 3. Concurrent access is serialized (e.g., with spinlocks).
```

**Justification:**
- Direct volatile pointer reads/writes are unavoidable when accessing hardware registers.
- Typed wrappers (`VolatileCell<T>`, `Uart<BASE>`) limit the surface and force explicit unsafe.
- Type-state (e.g., `Timer<BASE, Initialized>`) prevents unsafe before init.

#### Unsafe Blocks: Cycle Counters

**Location**: `core/time.rs`

**SAFETY Contract:**
```
// SAFETY: RDTSC (x86_64) and CNTVCT_EL0 (aarch64) are always available.
// Reading the cycle counter has no side effects and is safe from any context.
```

**Justification:**
- Cycle counters are special-purpose, read-only registers.
- No state is modified; only timing information is acquired.

#### Unsafe Blocks: Capability Operations

**Location**: `hal/devices/interrupts.rs`

**SAFETY Contract:**
```
// SAFETY: Caller provides IrqMaskCapability, proving authorization.
// BASE must point to valid interrupt controller registers.
```

**Justification:**
- Capabilities model authorization; capability-holder is responsible for correctness.
- Prevents unauthorized IRQ manipulation through type-level enforcement.

### Feature Preservation

All existing modules remain intact:

- **All scheduler types** (Round-Robin, Weighted RR, FIFO, LIFO, Cooperative, ...): ✓
- **All driver modules**: ✓
- **VFS, memory management, security**: ✓
- **All feature flags**: ✓ (no removal or renaming)

### Migration Strategy

**Phase 1** (current): Layered facades without removing existing code.
**Phase 2**: Gradually migrate callers to depend on `core` traits instead of direct HAL.
**Phase 3**: Advanced AOP macros for telemetry and performance monitoring.

### Example Usage

#### Type-Safe UART

```rust
use kernel::hal::devices::Uart;

// Create a UART at the standard x86 I/O base (0x3F8)
let mut uart: Uart<0x3F8> = Uart::new();
uart.send_byte(b'H');
uart.send_byte(b'i');

// Write via fmt::Write trait
use core::fmt::Write;
writeln!(uart, "Hello, {}", "world").ok();
```

#### Type-State Timer

```rust
use kernel::hal::devices::Timer;

// Create uninitialized
let timer_uninit: Timer<0x1000, Uninitialized> = Timer::new();

// Initialize (requires unsafe; caller asserts BASE is valid)
let timer: Timer<0x1000, Initialized> = unsafe { timer_uninit.init() };

// Safe to use now; type prevents accidental use-before-init
let count = timer.read_count();
```

#### AOP Logging

```rust
use aop_macros::log_entry;

#[log_entry(debug)]
fn process_interrupt() {
    // Code here
    // Macro expands to:
    // crate::core::log::log_event("debug", "[entry] process_interrupt");
    // ... function body ...
    // crate::core::log::log_event("debug", "[exit] process_interrupt");
}
```

### Testing

Tests are included in each module:
- `core/log.rs`: Log level composition tests.
- `core/time.rs`: Cycle counter and delay accuracy.
- `hal/mmio/typed_mmio.rs`: Volatile cell safety.
- `hal/devices/uart.rs`: UART send/receive behavior (requires serial harness).
- `hal/devices/timer.rs`: Timer conversion and state transitions.
- `hal/devices/interrupts.rs`: IRQ capability and priority handling.

---

**Last Updated:** May 7, 2026  
**Status:** Architecture scaffolding complete; migration in progress.
