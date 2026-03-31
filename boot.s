section .multiboot2
align 8
header_start:
    dd 0xe85250d6                ; magic
    dd 0                         ; architecture (0 = i386)
    dd header_end - header_start ; length
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start)) ; checksum

    ; End tag
    dw 0
    dw 0
    dd 8
header_end:

section .text
[bits 32] ; Force 32-bit instructions for the entry point
global _start
_start:
    cli
.hang:
    hlt
    jmp .hang