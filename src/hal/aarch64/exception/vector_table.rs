core::arch::global_asm!(
    r#"
.macro save_all
    sub sp, sp, #256
    stp x0, x1, [sp, #16 * 0]
    stp x2, x3, [sp, #16 * 1]
    stp x4, x5, [sp, #16 * 2]
    stp x6, x7, [sp, #16 * 3]
    stp x8, x9, [sp, #16 * 4]
    stp x10, x11, [sp, #16 * 5]
    stp x12, x13, [sp, #16 * 6]
    stp x14, x15, [sp, #16 * 7]
    stp x16, x17, [sp, #16 * 8]
    stp x18, x19, [sp, #16 * 9]
    stp x20, x21, [sp, #16 * 10]
    stp x22, x23, [sp, #16 * 11]
    stp x24, x25, [sp, #16 * 12]
    stp x26, x27, [sp, #16 * 13]
    stp x28, x29, [sp, #16 * 14]
    mrs x21, sp_el0
    mrs x22, elr_el1
    mrs x23, spsr_el1
    stp x30, x21, [sp, #16 * 15]
.endm

.macro restore_all
    ldp x30, x21, [sp, #16 * 15]
    msr sp_el0, x21
    ldp x28, x29, [sp, #16 * 14]
    ldp x26, x27, [sp, #16 * 13]
    ldp x24, x25, [sp, #16 * 12]
    ldp x22, x23, [sp, #16 * 11]
    ldp x20, x21, [sp, #16 * 10]
    ldp x18, x19, [sp, #16 * 9]
    ldp x16, x17, [sp, #16 * 8]
    ldp x14, x15, [sp, #16 * 7]
    ldp x12, x13, [sp, #16 * 6]
    ldp x10, x11, [sp, #16 * 5]
    ldp x8, x9, [sp, #16 * 4]
    ldp x6, x7, [sp, #16 * 3]
    ldp x4, x5, [sp, #16 * 2]
    ldp x2, x3, [sp, #16 * 1]
    ldp x0, x1, [sp, #16 * 0]
    add sp, sp, #256
    eret
.endm

.align 11
.global aarch64_vector_table
aarch64_vector_table:
    .align 7
    save_all
    mov x0, sp
    bl handle_sync
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_irq
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_fiq
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_serror
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_sync
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_irq
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_fiq
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_serror
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_sync
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_irq
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_fiq
    restore_all

    .align 7
    save_all
    mov x0, sp
    bl handle_serror
    restore_all

    .align 7
    b unhandled_exception
    .align 7
    b unhandled_exception
    .align 7
    b unhandled_exception
    .align 7
    b unhandled_exception
"#
);

extern "C" {
    fn aarch64_vector_table();
}

#[inline(always)]
pub(super) fn table_ptr() -> u64 {
    aarch64_vector_table as *const () as u64
}
