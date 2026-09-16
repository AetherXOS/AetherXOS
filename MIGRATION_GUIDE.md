## HAL to Core Traits Migration Guide

This guide explains how to migrate existing HAL-dependent code to use the new `core` traits and facades, as part of the Onion Architecture refactor.

### Overview

**Old Pattern (Direct HAL Dependencies):**
```rust
use kernel::hal::serial::write_raw;
use kernel::hal::uart::Uart;

fn log_something() {
    write_raw("message\n"); // Direct HAL dependency
}
```

**New Pattern (Core Traits):**
```rust
use kernel::core::log;

fn log_something() {
    log::info("message"); // Facade dependency; HAL is abstracted
}
```

### Rationale

- **Decoupling:** Core traits don't know about specific HAL implementations.
- **Testability:** Facades can be mocked for testing without hardware.
- **Portability:** Switching HAL implementations doesn't require refactoring callers.
- **Layering:** Ensures clear architectural boundaries (core → services → modules).

### Step-by-Step Migration

#### Step 1: Identify HAL Dependencies

Use `grep` to find direct HAL usage:
```bash
grep -r "use kernel::hal::" kernel/src/
grep -r "crate::hal::" kernel/src/
```

Common patterns to migrate:
- `hal::serial::write_raw()` → `core::log::log_event()`
- `hal::uart::Uart<BASE>` → Use through `services::` facade (for now)
- `hal::devices::*` → Already type-safe; gradually wrap in core traits

#### Step 2: Migrate to Core Facades

**Logging:**
```rust
// BEFORE
hal::serial::write_raw("Starting boot\n");

// AFTER
core::log::info("Starting boot");
```

**Available Log Levels:**
```rust
core::log::trace(msg);    // Most verbose
core::log::debug(msg);
core::log::info(msg);     // Default
core::log::warn(msg);
core::log::error(msg);    // Most critical
```

#### Step 3: Timing Operations

**Migrating Cycle Counting:**
```rust
// BEFORE
// (had to call arch-specific inline asm directly)

// AFTER
use core::time::cycle_count;

let start = cycle_count();
do_work();
let end = cycle_count();
let elapsed = end.saturating_sub(start);
```

#### Step 4: Use AOP Macros for Cross-Cutting Concerns

**Before (Manual):**
```rust
fn critical_operation() {
    write_raw("[ENTER] critical_operation\n");
    // ...work...
    write_raw("[EXIT] critical_operation\n");
}
```

**After (AOP):**
```rust
#[log_entry(debug)]
fn critical_operation() {
    // ...work...
    // Macro expands to insert entry/exit logs automatically
}
```

#### Step 5: Device Access Patterns

**Type-Safe UART (Already Migrated):**
```rust
// HAL devices expose const-generic types:
use hal::devices::Uart;

let mut uart: Uart<0x3F8> = Uart::new();
uart.send_byte(b'A');

// Write trait is implemented:
use core::fmt::Write;
writeln!(uart, "Hello").ok();
```

**Type-Safe Timer (Already Migrated):**
```rust
use hal::devices::{Timer, Initialized};

let timer: Timer<0x1000, Initialized> = unsafe { timer_uninit.init() };
let count = timer.read_count();
```

### Migration Checklist

- [ ] Find all direct `hal::` imports
- [ ] Replace `hal::serial::write_raw()` with `core::log::*()` calls
- [ ] Migrate timing code to use `core::time::cycle_count()`
- [ ] Add AOP macro attributes to functions with cross-cutting concerns
- [ ] Test with `cargo check --lib`
- [ ] Verify feature flags still work (e.g., `--features drivers`)
- [ ] Run tests: `cargo test --lib --target x86_64-pc-windows-msvc --no-run`

### Common Pitfalls

#### Pitfall 1: Forgetting Feature Gates

Some HAL modules are feature-gated. Ensure the facade gracefully handles feature-off scenarios:

```rust
// In services/vfs.rs:
#[cfg(feature = "vfs")]
pub use crate::modules::vfs::*;

// Caller code:
#[cfg(feature = "vfs")]
use services::vfs;
```

#### Pitfall 2: Circular Dependencies

Avoid `core` depending on `hal` or `modules`. The dependency graph should be:
```
modules/hal → core (one-way only)
services → modules/hal
core → (nothing except std::core, arch::*)
```

#### Pitfall 3: Logging from Signal Handlers

If logging from an IRQ handler, ensure the logging facade is safe for concurrent access:

```rust
// SAFETY: log_event calls hal::serial::write_raw, which must be
// interrupt-safe (e.g., no locks that might be held during IRQ).
#[irq_handler(priority = 10)]
fn timer_irq() {
    core::log::info("timer fired"); // Safe if serial is interrupt-safe
}
```

#### Pitfall 4: Macro Hygiene

Macros generated in `aop_macros` call `crate::core::log::*`. Ensure `crate` resolves correctly:

```rust
// In kernel/src/some_module.rs
#[log_entry]
fn my_fn() {
    // Macro expands to: crate::core::log::log_event(...)
    // This works because kernel/src/lib.rs exports pub mod core
}
```

### Performance Implications

- **Zero-Cost Abstractions:** Core facades are thin; no runtime overhead.
- **Const Generics:** `Uart<BASE>` eliminates runtime base address storage.
- **Inlining:** Most core functions are `#[inline]` for optimization.

### Testing Strategy

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use crate::core::log;

    #[test]
    fn test_logging_levels() {
        log::trace("test");
        log::debug("test");
        log::info("test");
        log::warn("test");
        log::error("test");
        // In real tests, capture output or use test harness
    }
}
```

#### Integration Tests

Test that migrated code still works with real hardware:

```bash
cargo test --lib --target x86_64-pc-windows-msvc --no-run
# Run the binary with emulator/hardware
```

### Gradual Migration Path

You don't need to migrate everything at once:

1. **Phase 1:** Add facades alongside existing code (done).
2. **Phase 2:** Migrate non-critical paths (modules that don't depend on others).
3. **Phase 3:** Migrate core scheduler/VFS code.
4. **Phase 4:** Remove stale HAL entry points (optional; keep for compatibility).

### Example: Complete Migration

**Before:**
```rust
// kernel/src/modules/process.rs
use crate::hal::serial;
use crate::hal::devices::Uart;

pub fn init_process(pid: u32) {
    serial::write_raw("Initializing PID ");
    // Manual formatting
    serial::write_raw(pid.to_string().as_str());
    serial::write_raw("\n");
    
    let mut uart: Uart<0x3F8> = Uart::new();
    uart.send_byte(b'P');
}
```

**After:**
```rust
// kernel/src/modules/process.rs
use crate::core::log;
use crate::hal::devices::Uart;
use core::fmt::Write;

#[log_entry(info)]
pub fn init_process(pid: u32) {
    log::info(&format!("Initializing PID {}", pid));
    
    let mut uart: Uart<0x3F8> = Uart::new();
    writeln!(uart, "P").ok();
}
```

### Validation

After migration:

```bash
# Ensure compilation
cargo check --lib

# Run tests
cargo test --lib --target x86_64-pc-windows-msvc --no-run

# Check for remaining direct hal:: imports
grep -r "use.*hal::" kernel/src/ | grep -v "hal::devices" | grep -v "hal::bridge"
# Should show only internal HAL code, not application code
```

---

**Last Updated:** May 7, 2026

**Next Steps:** Begin migrating modules under `kernel/src/modules/` that have minimal dependencies, then work up to scheduler and VFS.
