PLT Trampoline and Lazy Binding

Overview

This document describes the lazy PLT/GOT binding mechanism implemented in the
kernel loader.

- The loader registers JMP_SLOT/GOT slots discovered in a shared object's
  `.dynamic` (DT_JMPREL/DT_PLTREL) with a kernel `SharedObjectLoader`.
- For lazy resolution, the loader allocates an executable trampoline region in
  the process address space and writes a per-slot trampoline that performs the
  `RESOLVE_PLT` syscall.
- GOT/JMP_SLOT entries are initially patched to point to their corresponding
  trampolines. On first call, control transfers to the trampoline, which
  invokes the kernel resolver syscall with the slot virtual address.
- The kernel resolver looks up the registered symbol name for the slot, finds
  the resolved address from already-loaded shared objects (or local symbol
  tables), writes the resolved address into the GOT/JMP_SLOT entry, and
  returns it. The trampoline then jumps to the resolved address.

Trampoline encoding (x86_64)

The trampoline performs these operations (raw x86_64 bytes):

- `mov rax, RESOLVE_PLT`       ; syscall number in `rax`
- `mov rdi, slot_vaddr`        ; first argument: the GOT/JMP_SLOT address
- `xor rsi, rsi`               ; second arg unused
- `syscall`                    ; invoke kernel
- `jmp rax`                    ; jump to resolved address returned in `rax`

Security & Notes

- The trampoline is per-process and executable; the loader requests an
  executable mapping for it. This is necessary because the trampoline contains
  code executed in user context.
- The kernel resolver uses the registered PLT slot table to avoid relying on
  user-provided symbol names, removing a source of TOCTOU and trusting user
  memory.
- In future work, the trampoline could be made read-only after resolution to
  harden against tampering.

Reference

- See `tests/plt_trampoline_example.S` for a minimal illustrative snippet.
- Loader implementation: `src/kernel/module_loader.rs` (PLT registration and
  trampoline creation).
- Resolver syscall: `src/hal/x86_64/syscalls.rs` (handler `sys_resolve_plt`).
