use super::*;

#[inline(always)]
pub(crate) fn futex_key_from_ptr_or_hint(ptr: usize, key_hint: usize) -> u64 {
    if key_hint == 0 {
        ptr as u64
    } else {
        key_hint as u64
    }
}

#[allow(unused_variables)]
pub(crate) fn sys_futex_wait(user_word_ptr: usize, expected: usize, key_hint: usize) -> usize {
    SYSCALL_FUTEX_WAIT_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_IPC_FUTEX) {
        return err;
    }

    #[cfg(feature = "ipc")]
    {
        if expected > u32::MAX as usize {
            return invalid_arg();
        }

        with_user_read_bytes(user_word_ptr, FUTEX_WORD_BYTES, |slice| {
            let mut bytes = [0u8; FUTEX_WORD_BYTES];
            bytes.copy_from_slice(slice);
            let observed = u32::from_le_bytes(bytes);
            let key = futex_key_from_ptr_or_hint(user_word_ptr, key_hint);
            match crate::modules::ipc::futex::global().wait(key, observed, expected as u32) {
                crate::modules::ipc::futex::FutexWaitResult::Enqueued => 0,
                crate::modules::ipc::futex::FutexWaitResult::ValueMismatch => {
                    FUTEX_WAIT_VALUE_MISMATCH
                }
            }
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "ipc"))]
    {
        invalid_arg()
    }
}

#[allow(unused_variables)]
pub(crate) fn sys_futex_wake(user_word_ptr: usize, max_wake: usize, key_hint: usize) -> usize {
    SYSCALL_FUTEX_WAKE_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_IPC_FUTEX) {
        return err;
    }

    #[cfg(feature = "ipc")]
    {
        if max_wake == 0 {
            return 0;
        }

        if !user_readable_range_valid(user_word_ptr, FUTEX_WORD_BYTES) {
            return user_access_denied_arg();
        }

        let key = futex_key_from_ptr_or_hint(user_word_ptr, key_hint);
        crate::modules::ipc::futex::global().wake(key, max_wake)
    }

    #[cfg(not(feature = "ipc"))]
    {
        invalid_arg()
    }
}

#[allow(unused_variables)]
pub(crate) fn sys_upcall_register(
    irq: usize,
    entry_pc: usize,
    user_ctx: usize,
    flags: usize,
) -> usize {
    SYSCALL_UPCALL_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_IPC_UPCALL) {
        return err;
    }

    #[cfg(feature = "ipc_message_passing")]
    {
        if irq > u8::MAX as usize {
            return invalid_arg();
        }
        if flags > u32::MAX as usize {
            return invalid_arg();
        }
        if !upcall_entry_pc_valid(entry_pc) {
            return invalid_arg();
        }

        let Some(process_id) = current_process_id() else {
            return invalid_arg();
        };

        let pid = crate::interfaces::task::ProcessId(process_id);
        match register_upcall_with_owner_guard(
            irq as u8,
            pid,
            entry_pc as u64,
            user_ctx as u64,
            flags as u32,
        ) {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    #[cfg(not(feature = "ipc_message_passing"))]
    {
        invalid_arg()
    }
}

#[inline(always)]
fn register_upcall_with_owner_guard(
    irq: u8,
    pid: crate::interfaces::task::ProcessId,
    entry_pc: u64,
    user_ctx: u64,
    flags: u32,
) -> Result<(), usize> {
    let replaced =
        crate::modules::dispatcher::upcall::register_global(irq, pid, entry_pc, user_ctx, flags);
    if let Some(old) = replaced {
        let _ = crate::modules::dispatcher::upcall::register_global(
            irq,
            old.process_id,
            old.entry_pc,
            old.user_ctx,
            old.flags,
        );
        if old.process_id != pid {
            return Err(permission_denied_arg());
        }
        return Err(invalid_arg());
    }
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn sys_upcall_unregister(irq: usize) -> usize {
    SYSCALL_UPCALL_UNREGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_IPC_UPCALL) {
        return err;
    }

    #[cfg(feature = "ipc_message_passing")]
    {
        if irq > u8::MAX as usize {
            return invalid_arg();
        }

        let Some(process_id) = current_process_id() else {
            return invalid_arg();
        };

        if crate::modules::dispatcher::upcall::unregister_global_for_process(
            irq as u8,
            crate::interfaces::task::ProcessId(process_id),
        ) {
            0
        } else {
            invalid_arg()
        }
    }

    #[cfg(not(feature = "ipc_message_passing"))]
    {
        invalid_arg()
    }
}

#[allow(unused_variables)]
pub(crate) fn sys_upcall_query(irq: usize, ptr: usize, len: usize) -> usize {
    SYSCALL_UPCALL_QUERY_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_IPC_UPCALL) {
        return err;
    }

    #[cfg(feature = "ipc_message_passing")]
    {
        if irq > u8::MAX as usize {
            return invalid_arg();
        }

        let Some(process_id) = current_process_id() else {
            return invalid_arg();
        };

        let Some(entry) = crate::modules::dispatcher::upcall::resolve_global(irq as u8) else {
            return 0;
        };

        if entry.process_id != crate::interfaces::task::ProcessId(process_id) {
            return invalid_arg();
        }

        with_user_write_words(ptr, len, UPCALL_QUERY_WORDS, |out| {
            out[0] = entry.process_id.0;
            out[1] = entry.entry_pc as usize;
            out[2] = entry.user_ctx as usize;
            out[3] = entry.flags as usize;
            required_bytes(UPCALL_QUERY_WORDS)
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "ipc_message_passing"))]
    {
        invalid_arg()
    }
}

#[allow(unused_variables)]
pub(crate) fn sys_upcall_consume(ptr: usize, len: usize) -> usize {
    SYSCALL_UPCALL_CONSUME_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_IPC_UPCALL) {
        return err;
    }

    #[cfg(feature = "ipc_message_passing")]
    {
        let Some(process_id) = current_process_id() else {
            return invalid_arg();
        };

        let Some(delivery) = crate::modules::dispatcher::upcall::consume_global_for_process(
            crate::interfaces::task::ProcessId(process_id),
        ) else {
            return 0;
        };

        with_user_write_words(ptr, len, UPCALL_DELIVERY_WORDS, |out| {
            out[0] = delivery.irq as usize;
            out[1] = delivery.process_id.0;
            out[2] = delivery.entry_pc as usize;
            out[3] = delivery.user_ctx as usize;
            out[4] = delivery.flags as usize;
            required_bytes(UPCALL_DELIVERY_WORDS)
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "ipc_message_passing"))]
    {
        invalid_arg()
    }
}

#[allow(unused_variables)]
pub(crate) fn sys_upcall_inject_virtual_irq(irq: usize, user_ctx: usize, flags: usize) -> usize {
    SYSCALL_UPCALL_INJECT_VIRQ_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_IPC_UPCALL) {
        return err;
    }

    #[cfg(feature = "ipc_message_passing")]
    {
        if irq > u8::MAX as usize || flags > u32::MAX as usize {
            return invalid_arg();
        }

        let Some(process_id) = current_process_id() else {
            return invalid_arg();
        };

        if crate::modules::dispatcher::upcall::inject_global_virtual_irq(
            crate::interfaces::task::ProcessId(process_id),
            irq as u8,
            user_ctx as u64,
            flags as u32,
        ) {
            0
        } else {
            invalid_arg()
        }
    }

    #[cfg(not(feature = "ipc_message_passing"))]
    {
        invalid_arg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn upcall_owner_guard_rejects_cross_process_overwrite_and_rolls_back() {
        let irq = 0x61u8;
        let pid_a = crate::interfaces::task::ProcessId(10_001);
        let pid_b = crate::interfaces::task::ProcessId(10_002);

        let _ = crate::modules::dispatcher::upcall::unregister_global_for_process(irq, pid_a);
        let _ = crate::modules::dispatcher::upcall::unregister_global_for_process(irq, pid_b);

        assert_eq!(
            register_upcall_with_owner_guard(irq, pid_a, 0x4000, 0x11, 0),
            Ok(())
        );

        let denied = register_upcall_with_owner_guard(irq, pid_b, 0x5000, 0x22, 0);
        assert_eq!(denied, Err(permission_denied_arg()));

        let owner = crate::modules::dispatcher::upcall::resolve_global(irq)
            .expect("upcall entry should still belong to original owner after rollback");
        assert_eq!(owner.process_id, pid_a);
        assert_eq!(owner.entry_pc, 0x4000);
        assert_eq!(owner.user_ctx, 0x11);

        let _ = crate::modules::dispatcher::upcall::unregister_global_for_process(irq, pid_a);
    }

    #[test_case]
    fn upcall_owner_guard_rejects_same_process_overwrite_and_rolls_back() {
        let irq = 0x62u8;
        let pid = crate::interfaces::task::ProcessId(10_003);
        let _ = crate::modules::dispatcher::upcall::unregister_global_for_process(irq, pid);

        assert_eq!(
            register_upcall_with_owner_guard(irq, pid, 0x7000, 0x33, 0),
            Ok(())
        );

        let denied = register_upcall_with_owner_guard(irq, pid, 0x8000, 0x44, 0);
        assert_eq!(denied, Err(invalid_arg()));

        let owner = crate::modules::dispatcher::upcall::resolve_global(irq)
            .expect("upcall entry should remain original when same owner attempts overwrite");
        assert_eq!(owner.process_id, pid);
        assert_eq!(owner.entry_pc, 0x7000);
        assert_eq!(owner.user_ctx, 0x33);

        let _ = crate::modules::dispatcher::upcall::unregister_global_for_process(irq, pid);
    }
}
