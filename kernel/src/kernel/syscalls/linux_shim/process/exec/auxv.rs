use alloc::vec::Vec;
use super::super::exec_stack::{ExecveAuxEntry, ExecveAuxValue};
use super::env::{execve_aux_hwcap, execve_aux_platform};
use super::super::super::*;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
pub fn sanitized_phdr_aux_values(
    phdr_addr: usize,
    phent_size: usize,
    phnum: usize,
) -> Option<(usize, usize, usize)> {
    if phdr_addr == 0 || phent_size == 0 || phnum == 0 {
        return None;
    }
    if !(16..=4096).contains(&phent_size) {
        return None;
    }
    Some((phdr_addr, phent_size, phnum))
}

pub fn build_exec_auxv<'a>(
    entry_val: usize,
    base_addr: usize,
    phdr_addr: usize,
    phent_size: usize,
    phnum: usize,
    vdso_base: usize,
    execfn: &'a str,
    at_random: &'a [u8; 16],
) -> Vec<ExecveAuxEntry<'a>> {
    let phdr_aux = sanitized_phdr_aux_values(phdr_addr, phent_size, phnum);
    let uid = crate::modules::posix::process::getuid() as usize;
    let euid = crate::modules::posix::process::geteuid() as usize;
    let gid = crate::modules::posix::process::getgid() as usize;
    let egid = crate::modules::posix::process::getegid() as usize;
    let secure = usize::from(uid != euid || gid != egid);
    let (hwcap, hwcap2) = execve_aux_hwcap();
    let platform = execve_aux_platform();

    let mut auxv_entries = Vec::with_capacity(19);
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_ENTRY,
        value: ExecveAuxValue::Word(entry_val),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_PAGESZ,
        value: ExecveAuxValue::Word(crate::interfaces::memory::PAGE_SIZE_4K as usize),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_BASE,
        value: ExecveAuxValue::Word(base_addr),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_FLAGS,
        value: ExecveAuxValue::Word(0),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_UID,
        value: ExecveAuxValue::Word(uid),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_EUID,
        value: ExecveAuxValue::Word(euid),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_GID,
        value: ExecveAuxValue::Word(gid),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_EGID,
        value: ExecveAuxValue::Word(egid),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_SECURE,
        value: ExecveAuxValue::Word(secure),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_HWCAP,
        value: ExecveAuxValue::Word(hwcap),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_CLKTCK,
        value: ExecveAuxValue::Word(100),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_PLATFORM,
        value: ExecveAuxValue::CString(platform),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_RANDOM,
        value: ExecveAuxValue::Bytes(at_random),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_HWCAP2,
        value: ExecveAuxValue::Word(hwcap2),
    });

    if let Some((phdr, phent, num)) = phdr_aux {
        if crate::config::KernelConfig::exec_auxv_require_phdr_triplet() || true {
             auxv_entries.push(ExecveAuxEntry {
                key: EXECVE_AUXV_AT_PHDR,
                value: ExecveAuxValue::Word(phdr),
            });
            auxv_entries.push(ExecveAuxEntry {
                key: EXECVE_AUXV_AT_PHENT,
                value: ExecveAuxValue::Word(phent),
            });
            auxv_entries.push(ExecveAuxEntry {
                key: EXECVE_AUXV_AT_PHNUM,
                value: ExecveAuxValue::Word(num),
            });
        }
    }

    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_EXECFN,
        value: ExecveAuxValue::CString(execfn),
    });
    auxv_entries.push(ExecveAuxEntry {
        key: EXECVE_AUXV_AT_SYSINFO_EHDR,
        value: ExecveAuxValue::Word(vdso_base),
    });

    auxv_entries
}
