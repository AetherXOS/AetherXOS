; Bu dosya senin kernel'ının "kimlik kartı" olacak.
section .multiboot2
align 8
header_start:
    dd 0xe85250d6                ; Magic number (Multiboot2)
    dd 0                         ; Architecture (i386/x86_64)
    dd header_end - header_start ; Header length
    ; Checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))

    ; End tag
    dw 0
    dw 0
    dd 8
header_end:

section .text
global _start
_start:
    ; Kernel burada başlar. Şimdilik sadece dur (halt).
    cli
.hang:
    hlt
    jmp .hang