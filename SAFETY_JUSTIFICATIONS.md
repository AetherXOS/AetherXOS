## Safety Justification Guide

This document explains the safety rationale for every `unsafe` block in the kernel, supporting the Onion Architecture's "minimize & annotate" principle.

### Principle

Every `unsafe` block must have an adjacent `// SAFETY:` comment explaining:
1. **The invariant:** What condition must hold for the code to be correct.
2. **Caller responsibility:** What the caller must verify.
3. **Impact scope:** What breaks if the invariant is violated.

### MMIO & Volatile Access

#### File: `kernel/src/hal/mmio/typed_mmio.rs`

**Code:**
```rust
pub unsafe fn read_at(addr: usize) -> T {
    unsafe { core::ptr::read_volatile(addr as *const T) }
}
```

**SAFETY Justification:**
- **Invariant:** `addr` is a valid, readable MMIO memory location of type `T`.
- **Caller Responsibility:** The caller must ensure the address is within a valid mapped memory region and properly aligned for type `T`.
- **Impact if Violated:** Reading from invalid memory causes undefined behavior (UB) or CPU fault.
- **Why Unsafe is Necessary:** Volatile memory access is inherently unsafe because the compiler cannot verify the memory layout or validity at compile-time.

---

#### File: `kernel/src/hal/devices/uart.rs`

**Code:**
```rust
pub fn send_byte(&mut self, byte: u8) {
    unsafe {
        // SAFETY: BASE points to valid UART MMIO region (verified at struct creation).
        // Writing a single byte to the data register is atomic and race-free if no other
        // code concurrently accesses this UART device without locking.
        let data_reg = BASE as *mut u8;
        core::ptr::write_volatile(data_reg, byte);
    }
}
```

**SAFETY Justification:**
- **Invariant:** BASE address was verified valid during device initialization; no concurrent access.
- **Caller Responsibility:** Ensure mutual exclusion (spinlock, critical section, or single-threaded) when multiple cores/IRQs access the UART.
- **Impact if Violated:** Concurrent writes corrupt transmitted data.
- **Why Unsafe is Necessary:** Writing volatile state is unsafe; the compiler cannot check synchronization at compile-time.

---

#### File: `kernel/src/hal/devices/timer.rs`

**Code:**
```rust
pub unsafe fn init(self) -> Timer<BASE, Initialized> {
    let ctrl_reg = BASE as *mut u32;
    core::ptr::write_volatile(ctrl_reg, 1); // Enable timer
    
    Timer {
        _marker: PhantomData,
    }
}
```

**SAFETY Justification:**
- **Invariant:** BASE points to valid timer MMIO registers; called exactly once during boot.
- **Caller Responsibility:** Verify BASE address before calling; ensure no other code reconfigures the timer.
- **Impact if Violated:** Writing to invalid memory corrupts system state; double-init can cause timer overflow or state machine violations.
- **Why Unsafe is Necessary:** We cannot verify MMIO validity or boot-time guarantees at the function signature level.

---

### Cycle Counters

#### File: `kernel/src/core/time.rs`

**Code (x86_64):**
```rust
pub fn cycle_count() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: RDTSC is a user-accessible instruction available on all x86_64 processors.
        // Reading the Time Stamp Counter has no side effects and returns monotonic cycle counts
        // (modulo frequency scaling and CPU migration). This is safe to call from any context
        // (user, kernel, IRQ) without synchronization.
        unsafe { core::arch::x86_64::_rdtsc() }
    }
}
```

**SAFETY Justification:**
- **Invariant:** RDTSC instruction is always available on x86_64.
- **Caller Responsibility:** None; reading is always safe.
- **Impact if Violated:** (Not applicable; this cannot violate memory safety.)
- **Why Unsafe is Necessary:** Inline assembly requires unsafe; the CPU's RDTSC is provably safe by hardware design.

**Code (aarch64):**
```rust
pub fn cycle_count() -> u64 {
    let mut tsc: u64;
    unsafe {
        // SAFETY: CNTVCT_EL0 (virtual counter) is accessible from any exception level (EL0+).
        // Reading the counter is always safe; it has no side effects and uses inline asm
        // with `pure` and `nomem` flags to indicate no memory or system state is modified.
        core::arch::asm!(
            "mrs {}, CNTVCT_EL0",
            out(reg) tsc,
            options(nostack, nomem, pure)
        );
    }
    tsc
}
```

**SAFETY Justification:**
- **Invariant:** CNTVCT_EL0 exists on all aarch64 systems; reading has no side effects.
- **Caller Responsibility:** None; reading is always safe.
- **Impact if Violated:** (Not applicable; this is architecture-guaranteed.)
- **Why Unsafe is Necessary:** Inline asm requires unsafe; the instruction is provably safe by architecture spec.

---

### Interrupt Controller Capability Enforcement

#### File: `kernel/src/hal/devices/interrupts.rs`

**Code:**
```rust
pub fn disable_irq(&mut self, cap: &IrqMaskCapability) -> Result<(), ()> {
    let vector = cap.vector();
    
    unsafe {
        // SAFETY: The IrqMaskCapability proves authorization to manipulate this IRQ vector.
        // We write to the disable register for the specific vector, which is safe as long as:
        // 1. BASE points to valid interrupt controller registers (verified at struct creation).
        // 2. The caller has not already disabled this IRQ (capability system prevents double-disable).
        // Impact: Disabling an IRQ without servicing hardware may cause loss of events, but is not UB.
        let disable_offset = 0x20 + (vector.0 as usize * 4);
        let disable_reg = (BASE + disable_offset) as *mut u32;
        core::ptr::write_volatile(disable_reg, 0);
    }
    
    Ok(())
}
```

**SAFETY Justification:**
- **Invariant:** The caller provides a valid capability; BASE points to interrupt controller.
- **Caller Responsibility:** Do not call with a capability for a vector you don't own.
- **Impact if Violated:** Disabling an IRQ used by another component causes loss of events (system malfunction, not UB).
- **Why Unsafe is Necessary:** Volatile register writes require unsafe; capability-checking happens at compile-time via Rust types.

---

### Type-State Transitions

#### File: `kernel/src/hal/devices/interrupts.rs`

**Code:**
```rust
pub unsafe fn ack_irq(&mut self, vector: IrqVector) {
    let ack_reg = (BASE + 0xB0) as *mut u32;
    core::ptr::write_volatile(ack_reg, vector.0 as u32);
}
```

**SAFETY Justification:**
- **Invariant:** The IRQ has been serviced (hardware condition met); ACK will not cause loss of pending event.
- **Caller Responsibility:** Only call after handling the interrupt condition; do not call multiple times.
- **Impact if Violated:** Double-ACK may clear pending events or violate interrupt controller state machine.
- **Why Unsafe is Necessary:** We cannot verify at compile-time that the hardware condition has been met.

---

### AOP Macro Expansions

#### File: `aop_macros/src/lib.rs`

**Code:**
```rust
pub fn irq_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    // ... macro code ...
    let __irq_ts_start = crate::core::time::cycle_count();
    // ... user function ...
    let __irq_ts_end = crate::core::time::cycle_count();
}
```

**SAFETY Justification:**
- **Invariant:** `cycle_count()` is safe to call from any context (see `core/time.rs`).
- **Caller Responsibility:** Ensure the handler doesn't call blocking functions that depend on scheduling.
- **Impact if Violated:** (Not applicable; `cycle_count()` is provably safe.)
- **Why Unsafe is Necessary:** (Not explicitly unsafe in this case; macro expansion is safe.)

---

### Memory & Allocator Safety

#### File: `kernel/src/services/memory.rs`

**Code (Re-export):**
```rust
pub use crate::modules::allocators::*;
```

**SAFETY Justification:**
- **Invariant:** Allocators in the modules layer have their own safety contracts (see respective files).
- **Caller Responsibility:** Follow allocator-specific safety requirements (e.g., alignment, size).
- **Impact if Violated:** Memory corruption, use-after-free, or double-free.
- **Why Unsafe is Necessary:** (Delegated to underlying modules; see their safety docs.)

---

### Summary Table

| Component | Unsafe Scope | Invariant | Caller Responsibility |
|-----------|-------------|-----------|----------------------|
| `typed_mmio.rs` | Volatile reads/writes | BASE is valid MMIO | Provide valid address |
| `uart.rs` | Sending bytes | BASE is valid UART | Synchronize access |
| `timer.rs` | Hardware init | BASE is valid timer | Call once at boot |
| `time.rs` | RDTSC/CNTVCT | CPU instruction available | None (arch-guaranteed) |
| `interrupts.rs` | IRQ masking | Capability proves authorization | Follow capability semantics |

---

**Conclusion:** Every unsafe block in this architecture is justified by hardware constraints (MMIO, cycle counters, interrupt controllers) or capability-based type-level enforcement. All violations are documented; callers are responsible for upholding invariants that Rust cannot verify at compile-time.

---

**Last Updated:** May 7, 2026
