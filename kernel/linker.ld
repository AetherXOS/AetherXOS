/* 
 * AetherCore x86_64 Bare-Metal Linker Script
 * Optimized for Higher-Half architecture and the Limine Bootloader.
 */

ENTRY(_start)

/* 
 * Program Headers (PHDRS)
 * Explicitly defines memory segments and their permissions for the CPU and Bootloader.
 */
PHDRS {
    text    PT_LOAD    FLAGS(5);      /* R-X : Executable Code (Read + Execute) */
    rodata  PT_LOAD    FLAGS(4);      /* R-- : Read-Only Data */
    data    PT_LOAD    FLAGS(6);      /* RW- : Writable Data / BSS (Read + Write) */
    stack   PT_GNU_STACK FLAGS(6);    /* RW- : Non-Executable Stack (NX Stack Protection) */
}

SECTIONS {
    /* 
     * Higher-Half base virtual address. 
     * A +2MB (0x200000) offset is added to avoid colliding with physical memory 
     * mapped hardware regions (e.g., VGA buffer, BIOS data area) in the lower megabytes.
     */
    . = 0xffffffff80200000;

    /* Marks the start of the kernel for the Physical Memory Manager (PMM) */
    _kernel_start = .;

    /* Bootloader Requests and Multiboot Headers (KEEP prevents aggressive garbage collection) */
    .boot ALIGN(4K) : {
        KEEP(*(.multiboot2))
        KEEP(*(.limine_reqs)) 
    } :text

    /* Executable Code Section */
    .text ALIGN(4K) : {
        *(.text .text.*)
    } :text

    /* Read-Only Data (Constants, strings, etc.) */
    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    } :rodata

    /* Initialized Writable Data */
    .data ALIGN(4K) : {
        *(.data .data.*)
    } :data

    /* Uninitialized Writable Data (Zero-filled upon load) */
    .bss ALIGN(4K) : {
        *(COMMON)
        *(.bss .bss.*)
    } :data

    /* Marks the end of the kernel footprint for the Physical Memory Manager (PMM) */
    _kernel_end = .;

    /* 
     * Discard unwanted metadata and debug sections to reduce binary size.
     * WARNING: Do NOT discard .got, .dynamic, or .plt here. If the compiler generates
     * them, you must disable PIE (Position Independent Executable) in your Rust target JSON.
     */
    /DISCARD/ : {
        *(.comment)
        *(.note.gnu.build-id)
    }
}