# HAL Ownership Boundaries

## Purpose
This document defines strict ownership boundaries between architecture-specific HAL code and shared HAL code.

## Ownership Rules
- `kernel/src/hal/common/**` owns only generic mechanisms and reusable abstractions.
- `kernel/src/hal/x86_64/**` owns x86_64-specific constants, vector maps, register semantics, and handlers.
- `kernel/src/hal/aarch64/**` owns AArch64-specific exception classes, IRQ IDs, register semantics, and handlers.

## What Must Stay Architecture-Specific
- Exception class decoding tied to architecture encodings.
- IRQ/vector numeric maps that are platform or architecture specific.
- Register-level trap/interrupt state interpretation.
- APIC/GIC/PIC details and backend-specific control flows.

## What Can Be Shared
- Descriptor and classification helpers where the descriptor data is passed in.
- Generic IRQ registration iteration helpers.
- Generic snapshot/telemetry data containers.
- Generic storm-window and rate-tracking primitives.

## Dependency Direction
- Allowed: arch modules depend on common.
- Not allowed: common depends on arch modules.
- Not allowed: common stores architecture-owned constant tables.

## Refactor Checklist
- Before moving code to common, verify all architecture semantics are parameterized.
- Keep arch-owned tables in `hal/<arch>/...` and pass them into common helpers.
- Add compile-time or init-time assertion when table lengths matter.
- Validate both targets after refactor:
  - `cargo check --target x86_64-pc-windows-msvc -q`
  - `cargo check --target aarch64-unknown-none -q`

## Recent Applied Patterns
- Shared IRQ route registration helper in `hal/common/irq_registration.rs`.
- x86_64 IDT route table uses compile-time count assertion.
- AArch64 platform IRQ enable list remains arch-owned and uses shared iterator helper.
- AArch64 exception hotspot snapshot changed from tuple output to typed struct output.
