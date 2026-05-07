#![cfg(test)]

use crate::modules::posix::process;

#[test_case]
fn compile_smoke_fork_exec_wait() {
    // Compile-time smoke: ensure APIs are present and link correctly.
    let _ = process::fork();
    let _ = process::execve("/bin/true", &[], &[]);
    let _ = process::waitpid(0, true);
}
