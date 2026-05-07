use super::super::super::*;

pub fn validate_exec_entry_point(entry_val: usize) -> Result<(), usize> {
    if entry_val == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC));
    }
    Ok(())
}

pub fn validate_exec_handoff_contract(
    entry_val: usize,
    base_addr: usize,
    phdr_addr: usize,
    phent_size: usize,
    phnum: usize,
) -> Result<(), usize> {
    if !crate::config::KernelConfig::exec_auxv_enforce_handoff_contract() {
        return Ok(());
    }

    if entry_val == 0 || base_addr == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC));
    }

    let require_phdr_triplet = crate::config::KernelConfig::exec_auxv_require_phdr_triplet();
    let has_phdr_triplet = phdr_addr != 0 && phent_size != 0 && phnum != 0;
    if require_phdr_triplet && !has_phdr_triplet {
        return Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC));
    }

    if has_phdr_triplet {
        if !(16..=4096).contains(&phent_size) || phnum > 4096 {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC));
        }
        if phdr_addr < base_addr {
            return Err(linux_errno(crate::modules::posix_consts::errno::ENOEXEC));
        }
    }

    Ok(())
}
