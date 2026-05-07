use super::super::*;

#[derive(Clone, Copy)]
pub enum ExecveAuxValue<'a> {
    Word(usize),
    CString(&'a str),
    Bytes(&'a [u8]),
}

#[derive(Clone, Copy)]
pub struct ExecveAuxEntry<'a> {
    pub key: usize,
    pub value: ExecveAuxValue<'a>,
}

pub fn execve_stack_required_bytes(
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

fn validate_execve_auxv_entries(auxv_entries: &[ExecveAuxEntry<'_>]) -> Result<(), usize> {
    if auxv_entries
        .iter()
        .any(|entry| entry.key == EXECVE_AUXV_AT_NULL)
    {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }
    Ok(())
}

pub fn push_execve_user_word(
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

pub fn prepare_execve_user_stack(
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
