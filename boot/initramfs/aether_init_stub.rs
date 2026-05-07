// Minimal aether_init - PID 1 process
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Minimal syscall: exit(0) - SYS_exit_group = 231 on x86_64
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 231i64,  // SYS_exit_group
            in("rdi") 0i64,    // exit code 0
            options(noreturn)
        );
    }
}
