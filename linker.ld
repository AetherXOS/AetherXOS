/* AetherCore x86_64 Bare-Metal Linker Script
 * Loads at 1 MiB (Limine-compatible), statically linked, NX stack.
 */

ENTRY(_start)

PHDRS {
    text    PT_LOAD    FLAGS(5);      /* R-X : code              */
    rodata  PT_LOAD    FLAGS(4);      /* R-- : read-only data    */
    data    PT_LOAD    FLAGS(6);      /* RW- : writable data/BSS */
    stack   PT_GNU_STACK FLAGS(6);   /* RW- : non-executable stack (NX) */
}

SECTIONS {
    . = 1M;

    .boot ALIGN(4K) : {
        KEEP(*(.multiboot2))
    } :text

    .text ALIGN(4K) : {
        *(.text .text.*)
    } :text

    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    } :rodata

    .data ALIGN(4K) : {
        *(.data .data.*)
    } :data

    .bss ALIGN(4K) : {
        *(COMMON)
        *(.bss .bss.*)
    } :data

    /* ── Discard dynamic-linking infrastructure ─────────────────────────
     * These sections must not exist in a bare-metal static binary.
     * Their presence causes the ".dynamic section" ELF validation warning.
     */
    /DISCARD/ : {
        *(.comment)
        *(.eh_frame .eh_frame_hdr)
        *(.note.gnu.build-id)
        /* Dynamic linker/loader — forbidden in bare-metal kernel */
        *(.dynamic)
        *(.got)
        *(.got.plt)
        *(.plt .plt.*)
        *(.iplt .igot.plt)
        *(.interp)
        *(.gnu.version .gnu.version_r .gnu.version_d)
        *(.gnu.hash)
        *(.dynsym .dynstr)
        *(.rela.dyn .rela.plt .rel.dyn .rel.plt)
    }
}