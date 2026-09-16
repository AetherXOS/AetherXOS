## Phase 2: Selective Migration - Practical Strategies

**Status:** In Progress  
**Last Updated:** May 7, 2026

This document outlines concrete strategies for migrating existing AetherXOS code to use the new Onion Architecture facades. It includes real examples, decision trees, and refactoring patterns.

---

### 1. Boot-Time Logging Migration

**Goal:** Replace all `hal::serial::write_raw()` and `hal::serial::write_trace()` calls with `core::log::*()` equivalents.

**Why:** 
- Boot-time code is most visible and benefits most from structured logging
- Performance timing is critical during boot (cycle counting is built-in)
- Easy to validate changes (visual inspection of boot output)

#### Migration Pattern

**Before:**
```rust
use crate::hal::serial;

pub fn memory_init() {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    serial::write_raw("[EARLY] Memory init starting\n");
    
    // ... do work ...
    
    serial::write_raw("[EARLY] Memory init complete\n");
}
```

**After:**
```rust
use crate::core::log;

pub fn memory_init() {
    log::info("Memory init starting");
    
    // ... do work ...
    
    log::info("Memory init complete");
}
```

**Benefits:**
- Automatic timestamp and log level handling
- Conditional compilation removed (filtering done at runtime)
- Performance data available via `core::time::cycle_count()`

#### Step 1: Identify Candidates

Files with direct `hal::serial::*` usage:
```
kernel/src/interfaces/task/task.rs         - 8 calls
kernel/src/config/debug_macros_examples.rs - commented out (skip)
kernel/src/kernel/boot_health.rs           - ~5 calls (if it exists)
```

Use search:
```bash
grep -r "hal::serial::" kernel/src/ --include="*.rs" | grep -v "^Binary"
```

#### Step 2: Migrate Priority Files

1. **Highest:** `interfaces/task/task.rs` - Task creation is frequently traced
2. **High:** `kernel/boot_health.rs` - Boot-time diagnostics
3. **Medium:** `kernel/*/diagnostics.rs` - System diagnostics

#### Step 3: Replace Calls

Simple find-replace patterns:

| Old Pattern | New Pattern | Notes |
|-------------|-------------|-------|
| `write_raw("[EARLY] msg\n")` | `log::info("msg")` | Strip prefixes |
| `write_trace("fn", "event")` | `log::debug(&format!("{}: {}", "fn", "event"))` | Preserve structure |
| `write_error(msg)` | `log::error(msg)` | Direct mapping |
| `write_warn(msg)` | `log::warn(msg)` | Direct mapping |

---

### 2. Device Driver Migration

**Goal:** Replace ad-hoc MMIO access with typed `hal::devices::*` abstractions.

**Why:**
- Device code is modular and self-contained
- Type-safe patterns prevent register misuse
- Future extensibility (add I2C, SPI, etc.)

#### Candidates for Migration

1. **UART Driver** → `hal::devices::Uart<const BASE>`
   - Status: Already implemented
   - Usage: `send_byte()`, `Write` trait

2. **Timer Driver** → `hal::devices::Timer<const BASE, STATE>`
   - Status: Already implemented
   - Usage: Type-state prevents init errors

3. **Interrupt Controller** → `hal::devices::InterruptController<const BASE>`
   - Status: Already implemented
   - Usage: Capability tokens for authorization

#### Migration Example: UART

**Before:**
```rust
const UART_BASE: usize = 0x3F8;

pub fn send_char(c: u8) {
    let ptr = UART_BASE as *mut u8;
    unsafe {
        core::ptr::write_volatile(ptr, c);
    }
}

pub fn init() {
    // Manual register writes...
    let ptr = (UART_BASE + 0x04) as *mut u8;
    unsafe {
        core::ptr::write_volatile(ptr, 0x03);
    }
}
```

**After:**
```rust
use crate::hal::devices::Uart;

let mut uart: Uart<0x3F8> = Uart::new();

// Automatic with Write trait
use core::fmt::Write;
writeln!(uart, "Hello {}", 42).ok();
```

---

### 3. AOP Macro Integration

**Goal:** Add AOP macros to existing functions for automatic tracing and performance monitoring.

**Why:**
- Zero-overhead abstractions (macros are compile-time)
- Consistent cross-cutting concerns
- Easy to enable/disable per-function

#### Integration Pattern

**Before:**
```rust
pub fn critical_section() {
    let start = get_tsc();
    
    // ... work ...
    
    let end = get_tsc();
    if end - start > THRESHOLD {
        write_raw("SLOW CRITICAL SECTION\n");
    }
}
```

**After:**
```rust
use aop_macros::perf_trace;

#[perf_trace(threshold = 10000)]
pub fn critical_section() {
    // ... work ...
    // Macro automatically inserts timing and logging
}
```

#### Step-by-Step

1. **Identify high-value targets:**
   - Scheduler entry points (`schedule()`, `yield_cpu()`)
   - VFS operations (`open()`, `read()`, `write()`)
   - Memory allocation (`allocate()`, `deallocate()`)

2. **Add AOP annotations:**
   ```rust
   use aop_macros::{log_entry, perf_trace};
   
   #[log_entry(debug)]
   #[perf_trace(threshold = 50000)]
   pub fn schedule() {
       // Automatically logs entry/exit + performance
   }
   ```

3. **Verify no breakage:**
   ```bash
   cargo check --lib
   cargo test --lib --target x86_64-pc-windows-msvc --no-run
   ```

---

### 4. Runtime Log Level Filtering

**Goal:** Enable selective verbosity via `core::log_filter`.

**Why:**
- Reduce boot-time spam
- Enable detailed tracing for specific subsystems
- Dynamic reconfiguration possible

#### Usage Examples

```rust
use crate::core::log_filter::{LogLevelValue, set_subsystem_log_level, should_log_at_level};

// Globally, only show warnings and above
set_global_log_level(LogLevelValue::Warn);

// But for scheduler debugging, enable trace
set_subsystem_log_level("scheduler", LogLevelValue::Trace);

// In code:
if should_log_at_level("debug") {
    log::debug("detailed message");
}
```

#### Integration Points

1. **Boot-time config:** Set from bootloader parameters
2. **Diagnostics menu:** Allow runtime adjustment
3. **Macro support:** Future macros can check filtering

---

### 5. Decision Tree: When to Migrate

```
Is the code already working?
├─ YES: Does it need restructuring for clarity?
│        ├─ NO: Don't migrate (preserve working code)
│        └─ YES: Is it modular and testable?
│                 ├─ YES: Migrate to use facades
│                 └─ NO: Refactor first, then migrate
└─ NO: Is it critical functionality?
       ├─ YES: Fix first, then consider migration
       └─ NO: Document as future work
```

---

### 6. Migration Checklist

For each file to be migrated:

- [ ] Identify all HAL dependencies (`grep "hal::"`)
- [ ] Document current behavior (test, manual)
- [ ] Create facade equivalents (log, devices, etc.)
- [ ] Replace calls incrementally
- [ ] Verify with `cargo check --lib`
- [ ] Run tests: `cargo test --lib --target x86_64-pc-windows-msvc --no-run`
- [ ] Manual testing if hardware available
- [ ] Code review (check for edge cases)
- [ ] Update documentation

---

### 7. Common Pitfalls & Solutions

#### Pitfall 1: Circular Dependencies

**Problem:**
```rust
// core/log.rs wants to call hal::serial::write_raw
// hal/serial.rs wants to use core traits
```

**Solution:** Create a HAL bridge that implements core traits without importing from `core`.

#### Pitfall 2: Feature Gate Mismatch

**Problem:**
```rust
#[cfg(feature = "vfs")]
use crate::services::vfs;  // But code runs when feature is OFF
```

**Solution:** Ensure all uses are guarded with the same `#[cfg]`.

#### Pitfall 3: Performance Regression

**Problem:** Migrating to facades accidentally adds overhead (e.g., format!() calls).

**Solution:** Use `inline` hints and verify with `cargo asm` or benchmarks.

#### Pitfall 4: Incomplete Migration

**Problem:** Some callers still use old API; others use new.

**Solution:** Deprecate old API early, or provide a compat layer.

---

### 8. Recommended Migration Order

1. **Week 1:** Boot logger examples + tests
2. **Week 2:** Migrate interfaces/task/task.rs
3. **Week 3:** Migrate device drivers (UART, Timer, Interrupts)
4. **Week 4:** Add AOP macros to scheduler
5. **Week 5:** Migrate VFS logging
6. **Week 6:** Comprehensive testing + doc updates

---

### 9. Success Metrics

- [ ] **Zero test failures** after each migration
- [ ] **No performance regression** in boot time (measure cycle counts)
- [ ] **Clear audit trail**: Every `unsafe` block justified
- [ ] **Feature preservation**: All 20+ schedulers still available
- [ ] **Code coverage**: AOP macros exercised in tests

---

### 10. Resources

- `ARCHITECTURE.md` - Layer design and philosophy
- `SAFETY_JUSTIFICATIONS.md` - Every unsafe block explained
- `aop_macro_tests.rs` - Macro expansion test suite
- `boot_logger.rs` - Real-world example of structured logging

---

**Next Actions:**

1. Run `cargo check --lib` to verify Phase 2 setup is correct
2. Start with `interfaces/task/task.rs` migration (highest value)
3. Add AOP macros to scheduler functions
4. Create migration branch for review

---

**Approval:** Ready for Phase 2 execution upon user confirmation.
