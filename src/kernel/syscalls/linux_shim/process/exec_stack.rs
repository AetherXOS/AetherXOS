#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
use super::super::*;

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
#[derive(Clone, Copy)]
pub(crate) enum ExecveAuxValue<'a> {
    Word(usize),
    CString(&'a str),
    Bytes(&'a [u8]),
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
#[derive(Clone, Copy)]
pub(crate) struct ExecveAuxEntry<'a> {
    pub key: usize,
    pub value: ExecveAuxValue<'a>,
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
pub(crate) fn execve_stack_required_bytes(
    argv: &[alloc::string::String],
    envp: &[alloc::string::String],
    auxv_entries: &[ExecveAuxEntry<'_>],
) -> Option<usize> {
    let word_size = core::mem::size_of::<usize>();
    let strings_bytes = argv
        .iter()
        .chain(envp.iter())
        .try_fold(0usize, |acc, s| acc.checked_add(s.len().checked_add(1)?))?;
    let aux_string_bytes =
        auxv_entries
            .iter()
            .try_fold(0usize, |acc, entry| match entry.value {
                ExecveAuxValue::Word(_) => Some(acc),
                ExecveAuxValue::CString(s) => acc.checked_add(s.len().checked_add(1)?),
                ExecveAuxValue::Bytes(bytes) => acc.checked_add(bytes.len()),
            })?;
    let aux_words = auxv_entries.len().checked_mul(2)?.checked_add(2)?;
    let pointer_words = argv
        .len()
        .checked_add(1)?
        .checked_add(envp.len())?
        .checked_add(1)?
        .checked_add(aux_words)?
        .checked_add(1)?;
    let pointer_bytes = pointer_words.checked_mul(word_size)?;
    strings_bytes
        .checked_add(aux_string_bytes)?
        .checked_add(pointer_bytes)?
        .checked_add(word_size.saturating_sub(1))
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
fn validate_execve_auxv_entries(auxv_entries: &[ExecveAuxEntry<'_>]) -> Result<(), usize> {
    if auxv_entries
        .iter()
        .any(|entry| entry.key == EXECVE_AUXV_AT_NULL)
    {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
pub(crate) fn push_execve_user_word(
    sp: &mut u64,
    word_size: u64,
    value: usize,
) -> Result<(), usize> {
    let Some(next_sp) = sp.checked_sub(word_size) else {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    };
    *sp = next_sp;
    if with_user_write_words(*sp as usize, core::mem::size_of::<usize>(), 1, |words| {
        words[0] = value;
    })
    .is_err()
    {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }
    Ok(())
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
fn push_execve_user_c_string(sp: &mut u64, value: &str) -> Result<usize, usize> {
    let bytes = value.as_bytes();
    let len = bytes.len().saturating_add(1);
    let Some(next_sp) = sp.checked_sub(len as u64) else {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    };
    *sp = next_sp;
    if with_user_write_bytes(*sp as usize, len, |dst| {
        dst[..bytes.len()].copy_from_slice(bytes);
        dst[bytes.len()] = 0;
    })
    .is_err()
    {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }
    Ok(*sp as usize)
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
fn push_execve_user_bytes(sp: &mut u64, value: &[u8]) -> Result<usize, usize> {
    let len = value.len();
    let Some(next_sp) = sp.checked_sub(len as u64) else {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    };
    *sp = next_sp;
    if with_user_write_bytes(*sp as usize, len, |dst| {
        dst.copy_from_slice(value);
    })
    .is_err()
    {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }
    Ok(*sp as usize)
}

#[cfg(any(
    all(
        not(feature = "linux_compat"),
        feature = "posix_process",
        feature = "process_abstraction",
        feature = "vfs",
        feature = "posix_mman"
    ),
    test
))]
pub(crate) fn prepare_execve_user_stack(
    stack_start: u64,
    stack_size: u64,
    argv: &[alloc::string::String],
    envp: &[alloc::string::String],
    auxv_entries: &[ExecveAuxEntry<'_>],
) -> Result<u64, usize> {
    validate_execve_auxv_entries(auxv_entries)?;
    let Some(required) = execve_stack_required_bytes(argv, envp, auxv_entries) else {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    };
    if required as u64 > stack_size {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let word_size = core::mem::size_of::<usize>() as u64;
    let Some(mut sp) = stack_start.checked_add(stack_size) else {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    };

    let mut arg_ptrs: alloc::vec::Vec<usize> = alloc::vec::Vec::new();
    for s in argv.iter().rev() {
        arg_ptrs.push(push_execve_user_c_string(&mut sp, s)?);
    }

    let mut env_ptrs: alloc::vec::Vec<usize> = alloc::vec::Vec::new();
    for s in envp.iter().rev() {
        env_ptrs.push(push_execve_user_c_string(&mut sp, s)?);
    }

    let mut resolved_auxv: alloc::vec::Vec<(usize, usize)> = alloc::vec::Vec::new();
    for entry in auxv_entries.iter().rev() {
        let value = match entry.value {
            ExecveAuxValue::Word(word) => word,
            ExecveAuxValue::CString(s) => push_execve_user_c_string(&mut sp, s)?,
            ExecveAuxValue::Bytes(bytes) => push_execve_user_bytes(&mut sp, bytes)?,
        };
        resolved_auxv.push((entry.key, value));
    }

    sp &= !(word_size - 1);

    for ptr in arg_ptrs.iter().rev() {
        push_execve_user_word(&mut sp, word_size, *ptr)?;
    }
    push_execve_user_word(&mut sp, word_size, 0)?;

    for ptr in env_ptrs.iter().rev() {
        push_execve_user_word(&mut sp, word_size, *ptr)?;
    }
    push_execve_user_word(&mut sp, word_size, 0)?;

    for &(key, value) in resolved_auxv.iter().rev() {
        push_execve_user_word(&mut sp, word_size, value)?;
        push_execve_user_word(&mut sp, word_size, key)?;
    }
    push_execve_user_word(&mut sp, word_size, 0)?;
    push_execve_user_word(&mut sp, word_size, EXECVE_AUXV_AT_NULL)?;
    push_execve_user_word(&mut sp, word_size, arg_ptrs.len())?;

    Ok(sp)
}

#[cfg(any(all(not(feature = "linux_compat"), test), test))]
fn execve_stack_layout_indexes(
    argv_len: usize,
    envp_len: usize,
    auxv_len: usize,
) -> (usize, usize, usize) {
    let aux_entry_index = 1;
    let envp_null_index = aux_entry_index + (auxv_len * 2) + 2;
    let argv_null_index = envp_null_index + envp_len + 1;
    let first_argv_index = argv_null_index + argv_len;
    (aux_entry_index, envp_null_index, first_argv_index)
}

#[cfg(any(all(not(feature = "linux_compat"), test), test))]
fn execve_stack_pointer_indexes(
    argv_len: usize,
    envp_len: usize,
    auxv_len: usize,
    argv_index: usize,
    envp_index: usize,
) -> (usize, usize) {
    let (_, envp_null_index, first_argv_index) =
        execve_stack_layout_indexes(argv_len, envp_len, auxv_len);
    let env_index = envp_null_index + envp_len.saturating_sub(envp_index);
    let arg_index = first_argv_index.saturating_sub(argv_index);
    (arg_index, env_index)
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[path = "exec_stack/tests.rs"]
mod tests;
