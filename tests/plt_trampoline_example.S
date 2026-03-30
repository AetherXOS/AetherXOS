/*
 Minimal x86_64 user-space trampoline example (assembly)
 This file is illustrative only — it shows how a PLT/JMP_SLOT lazy trampoline
 can be invoked. In the loader we install per-slot trampolines in kernel-space
 and patch the GOT/JMP_SLOT entries to point to them. The trampoline issues the
 `RESOLVE_PLT` syscall with the slot address in `rdi`, then jumps to the
 resolved address returned in `rax`.

 A simple user-space invocation of an external function would look like this:

     extern_func_call:
         call [got_slot]    ; indirect call through GOT/JMP_SLOT

 Where `got_slot` is a 64-bit memory entry initially pointing at the kernel
 trampoline. The kernel trampoline will perform the syscall to resolve the
 symbol and rewrite the GOT slot with the final address before transferring
 control.

 For completeness, here's a minimal trampoline encoding that matches the kernel
 trampoline layout (little-endian immediates shown as comments):

   ; mov rax, RESOLVE_PLT
   48 b8 <imm64>
   ; mov rdi, <slot_vaddr>
   48 bf <imm64>
   ; xor rsi, rsi
   48 31 f6
   ; syscall
   0f 05
   ; jmp rax
   ff e0

 Note: To build a real test, compile a small ELF with an undefined symbol and
 let the kernel loader set up GOT/PLT entries as implemented in
 `module_loader.rs`.
*/

    .section .text
    .global _start
_start:
    /* This example intentionally left non-functional as an assembly unit test.
       It documents how the lazy PLT works and how user code ends up calling
       through a GOT slot that the loader patches. */
    hlt
