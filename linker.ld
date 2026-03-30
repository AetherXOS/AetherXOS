/* HyperCore x86_64 Linker Script - Improved */
OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)

ENTRY(_start)

SECTIONS {
    /* Set current address to 1MB */
    . = 1M;

    /* Multiboot2 Header - MUST be at the beginning and 8-byte aligned */
    .multiboot2 ALIGN(8) : {
        KEEP(*(.multiboot2))
    }

    /* Text Section - Executable Code */
    /* Aligning to 4KB for Page Protection (Read/Execute) */
    .text ALIGN(4K) : {
        *(.text .text.*)
    }

    /* Read-Only Data (Constants, Strings) */
    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    }

    /* Initialized Data (Global Variables) */
    .data ALIGN(4K) : {
        *(.data .data.*)
    }

    /* Uninitialized Data (Stack, BSS) */
    .bss ALIGN(4K) : {
        *(COMMON)
        *(.bss .bss.*)
    }

    /* Discard unnecessary sections that may cause bloat or issues */
    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.note.gnu.build-id)
    }
}