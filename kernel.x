/* HyperCore x86_64 Linker Script with Multiboot2 Support */

ENTRY(_start)

SECTIONS {
    /* Start of executable - required for Multiboot2 */
    . = 0x100000;
    
    /* Multiboot2 header MUST be in first 32KB */
    .multiboot2 : {
        KEEP(*(.multiboot2))
    }
    
    /* Text and Read-Only Data */
    .text : {
        *(.text .text.*)
        *(.rodata .rodata.*)
    }
    
    /* Initialized Data */
    .data : {
        *(.data .data.*)
    }
    
    /* Uninitialized Data */
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
}
