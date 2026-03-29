fn join_parts(parts: &[&str]) -> String {
    parts.join("")
}

pub fn runtime_state_source() -> String {
    include_str!("templates/runtime_state.c.txt").to_string()
}

pub fn crt0_source() -> String {
    include_str!("templates/crt0.S.txt").to_string()
}

pub fn auxv_runtime_source() -> String {
    include_str!("templates/auxv_runtime.c.txt").to_string()
}

pub fn env_runtime_source() -> String {
    include_str!("templates/env_runtime.c.txt").to_string()
}

pub fn runtime_syscall_source() -> String {
    include_str!("templates/runtime_syscall.c.txt").to_string()
}

pub fn runtime_entry_source() -> String {
    include_str!("templates/runtime_entry.c.txt").to_string()
}

pub fn runtime_probe_source() -> String {
    include_str!("templates/runtime_probe.c.txt").to_string()
}

pub fn runtime_smoke_source() -> String {
    include_str!("templates/runtime_smoke.c.txt").to_string()
}

pub fn libc_state_source() -> String {
    include_str!("templates/libc_state.c.txt").to_string()
}

pub fn startup_runtime_source() -> String {
    include_str!("templates/startup_runtime.c.txt").to_string()
}

pub fn memory_runtime_source() -> String {
    include_str!("templates/memory_runtime.c.txt").to_string()
}

pub fn string_runtime_source() -> String {
    include_str!("templates/string_runtime.c.txt").to_string()
}

pub fn errno_runtime_source() -> String {
    join_parts(&[
        include_str!("templates/errno_runtime/errno_env_stdio.c.txt"),
        include_str!("templates/errno_runtime/dir_signal.c.txt"),
        include_str!("templates/errno_runtime/exit_assert.c.txt"),
    ])
}

pub fn libc_syscall_source() -> String {
    join_parts(&[
        include_str!("templates/libc_syscall/prologue.c.txt"),
        include_str!("templates/libc_syscall/net.c.txt"),
        include_str!("templates/libc_syscall/fs.c.txt"),
        include_str!("templates/libc_syscall/process_time.c.txt"),
        include_str!("templates/libc_syscall/special_fds.c.txt"),
        include_str!("templates/libc_syscall/memory_exec.c.txt"),
    ])
}
