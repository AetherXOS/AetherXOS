use super::super::*;
use crate::interfaces::cpu::CpuRegisters;
use crate::kernel::syscalls::with_user_write_bytes;
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref PDEATHSIG_BY_TID: Mutex<BTreeMap<usize, i32>> = Mutex::new(BTreeMap::new());
}

#[inline(always)]
fn current_tid() -> Option<usize> {
    unsafe { crate::kernel::cpu_local::CpuLocal::try_get() }
        .map(|cpu| cpu.current_task.load(core::sync::atomic::Ordering::Relaxed) as usize)
}

/// `arch_prctl(2)` — x86_64 spesifik thread state ayarları.
/// Glibc tarafından TLS (FS register) setlemek için kullanılır.
pub fn sys_linux_arch_prctl(code: usize, addr: usize) -> usize {
    use crate::kernel::syscalls::syscalls_consts::linux::arch_prctl;

    match code {
        arch_prctl::ARCH_SET_FS => {
            // Memory check: kullanıcı geçersiz bir adres vermesin.
            // addr 0 olabilir (bazı kütüphanelerde resetleme için).
            if addr != 0 && (addr >= crate::hal::syscalls_consts::USER_SPACE_TOP_EXCLUSIVE) {
                return linux_inval();
            }

            // FS register'ını güncel MSR üzerinden setle.
            // Bu, mevcut task'ın TLS pointer'ıdır.
            crate::hal::cpu::ArchCpuRegisters::write_tls_base(addr as u64);

            // Ayrıca scheduler'daki mevcut task'ın snapshot'ına da kaydetmeliyiz.
            if let Some(cpu) = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() } {
                let tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
                if let Some(task) =
                    crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid as usize))
                {
                    task.lock().user_tls_base = addr as u64;
                }
            }
            0
        }
        arch_prctl::ARCH_GET_FS => {
            if addr == 0 {
                return linux_fault();
            }
            let fs_base = crate::hal::cpu::ArchCpuRegisters::read_tls_base();
            let ptr = UserPtr::<usize>::new(addr);
            if let Err(_) = ptr.write(&(fs_base as usize)) {
                return linux_fault();
            }
            0
        }
        _ => linux_inval(),
    }
}

/// `prctl(2)` — Process operasyonları.
pub fn sys_linux_prctl(
    option: usize,
    arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> usize {
    match option {
        linux::PR_SET_NAME => {
            let name_ptr = UserPtr::<u8>::new(arg2);
            let name = match read_user_c_string(name_ptr.addr, linux::PR_NAME_MAX) {
                Ok(s) => s,
                Err(e) => return e,
            };

            if let Some(tid) = current_tid() {
                if let Some(task_arc) =
                    crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid))
                {
                    task_arc.lock().name = name;
                }
            }
            0
        }
        linux::PR_GET_NAME => {
            if arg2 == 0 {
                return linux_fault();
            }

            if let Some(tid) = current_tid() {
                if let Some(task_arc) =
                    crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid))
                {
                    let name = task_arc.lock().name.clone();
                    let name_bytes = name.as_bytes();
                    let len = core::cmp::min(name_bytes.len(), linux::PR_NAME_MAX - 1);

                    let _ = with_user_write_bytes(arg2, len + 1, |dst| {
                        dst[..len].copy_from_slice(&name_bytes[..len]);
                        dst[len] = 0;
                        0
                    });
                }
            }
            0
        }
        linux::PR_SET_PDEATHSIG => {
            let sig = arg2 as i32;
            if !(0..=64).contains(&sig) {
                return linux_inval();
            }
            let Some(tid) = current_tid() else {
                return linux_errno(crate::modules::posix_consts::errno::ESRCH);
            };
            let mut table = PDEATHSIG_BY_TID.lock();
            if sig == 0 {
                table.remove(&tid);
            } else {
                table.insert(tid, sig);
            }
            0
        }
        linux::PR_GET_PDEATHSIG => {
            let out_ptr = UserPtr::<i32>::new(arg2);
            if out_ptr.is_null() {
                return linux_fault();
            }
            let Some(tid) = current_tid() else {
                return linux_errno(crate::modules::posix_consts::errno::ESRCH);
            };
            let sig = *PDEATHSIG_BY_TID.lock().get(&tid).unwrap_or(&0);
            match out_ptr.write(&sig) {
                Ok(()) => 0,
                Err(e) => e,
            }
        }
        _ => linux_inval(),
    }
}

pub fn sys_linux_sigaltstack(
    ss_ptr: UserPtr<types::LinuxSigaltstack>,
    old_ss_ptr: UserPtr<types::LinuxSigaltstack>,
) -> usize {
    let tid = unsafe { crate::kernel::cpu_local::CpuLocal::get().current_task_id() };
    let Some(task_arc) = crate::kernel::task::get_task(tid) else {
        return linux_inval();
    };

    if !old_ss_ptr.is_null() {
        let mut old = types::LinuxSigaltstack::default();
        if let Some(stack) = task_arc.lock().signal_stack {
            old.ss_sp = stack.ss_sp;
            old.ss_flags = stack.ss_flags;
            old.ss_size = stack.ss_size;
        }
        let _ = old_ss_ptr.write(&old);
    }

    if !ss_ptr.is_null() {
        let ss = match ss_ptr.read() {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut task = task_arc.lock();
        task.signal_stack = Some(crate::interfaces::task::SignalStack {
            ss_sp: ss.ss_sp,
            ss_flags: ss.ss_flags,
            ss_size: ss.ss_size,
        });
    }

    0
}

pub fn sys_linux_set_tid_address(tidptr: usize) -> usize {
    // CLONE_CHILD_CLEARTID için kullanılan adres.
    if let Some(cpu) = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() } {
        let tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
        if let Some(task) =
            crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid as usize))
        {
            task.lock().clear_child_tid = tidptr;
        }
        return tid as usize;
    }
    linux_errno(crate::modules::posix_consts::errno::ESRCH)
}

pub fn sys_linux_personality(_persona: usize) -> usize {
    0 // PER_LINUX
}

pub fn sys_linux_getpriority(_which: i32, _who: i32) -> usize {
    20 // Default priority
}

pub fn sys_linux_setpriority(_which: i32, _who: i32, _prio: i32) -> usize {
    0
}
