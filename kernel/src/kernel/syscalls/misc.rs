use super::*;
use core::sync::atomic::Ordering;


pub(crate) fn sys_print(ptr: usize, len: usize) -> usize {
    SYSCALL_PRINT_CALLS.fetch_add(1, Ordering::Relaxed);

    with_user_read_bounded_bytes(ptr, len, MAX_PRINT_LEN, |slice| {
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::klog_info!("USER: {}", s);
            len
        } else {
            invalid_arg()
        }
    })
    .unwrap_or_else(|err| err)
}

pub(crate) fn sys_get_abi_info(ptr: usize, len: usize) -> usize {
    SYSCALL_ABI_INFO_CALLS.fetch_add(1, Ordering::Relaxed);

    with_user_write_words(
        ptr,
        len,
        SYSCALL_ABI_INFO_WORDS,
        |out| {
            out.copy_from_slice(&[
            SYSCALL_ABI_MAGIC,
            SYSCALL_ABI_VERSION_MAJOR,
            SYSCALL_ABI_VERSION_MINOR,
            SYSCALL_ABI_VERSION_PATCH,
            SYSCALL_ABI_MIN_COMPAT_MAJOR,
            nr::GET_ABI_INFO,
            SYSCALL_ABI_FLAG_STABLE,
            ]);
            required_bytes(SYSCALL_ABI_INFO_WORDS)
        },
    )
    .unwrap_or_else(|err| err)
}
