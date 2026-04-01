use super::*;

pub(crate) fn sys_get_process_launch_context(process_id: usize, ptr: usize, len: usize) -> usize {
    SYSCALL_PROCESS_LAUNCH_CTX_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        let Some(ctx) = crate::kernel::launch::process_launch_context(process_id) else {
            return invalid_arg();
        };

        write_launch_context_response(
            ptr,
            len,
            ctx.process_id,
            ctx.task_id,
            ctx.entry,
            ctx.image_pages,
            ctx.image_segments,
            ctx.mapped_regions,
            ctx.mapped_pages,
            ctx.cr3,
            required_bytes(PROCESS_LAUNCH_CTX_WORDS),
        )
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_claim_next_launch_context(ptr: usize, len: usize) -> usize {
    SYSCALL_PROCESS_CLAIM_CTX_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        let Some(ctx) = crate::kernel::launch::claim_next_launch_context() else {
            return 0;
        };
        write_launch_context_response(
            ptr,
            len,
            ctx.process_id,
            ctx.task_id,
            ctx.entry,
            ctx.image_pages,
            ctx.image_segments,
            ctx.mapped_regions,
            ctx.mapped_pages,
            ctx.cr3,
            ctx.process_id.0,
        )
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_ack_launch_context(process_id: usize, success_flag: usize) -> usize {
    SYSCALL_PROCESS_ACK_CTX_CALLS.fetch_add(1, Ordering::Relaxed);

    let Some(mode) = BinarySwitch::from_usize(success_flag) else {
        return invalid_arg();
    };
    let success = mode.is_enabled();

    #[cfg(feature = "process_abstraction")]
    {
        if crate::kernel::launch::acknowledge_launch_context(process_id, success) {
            0
        } else {
            invalid_arg()
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_get_launch_context_stage(process_id: usize) -> usize {
    SYSCALL_PROCESS_CTX_STAGE_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        crate::kernel::launch::launch_context_stage(process_id).unwrap_or(usize::MAX)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_consume_ready_launch_context(ptr: usize, len: usize) -> usize {
    SYSCALL_PROCESS_CONSUME_CTX_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        let Some(ctx) = crate::kernel::launch::consume_ready_launch_context() else {
            return 0;
        };
        write_launch_context_response(
            ptr,
            len,
            ctx.process_id,
            ctx.task_id,
            ctx.entry,
            ctx.image_pages,
            ctx.image_segments,
            ctx.mapped_regions,
            ctx.mapped_pages,
            ctx.cr3,
            ctx.process_id.0,
        )
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_execute_ready_launch_context() -> usize {
    SYSCALL_PROCESS_EXECUTE_CTX_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        crate::kernel::launch::execute_ready_launch_context_on_current_cpu()
            .map(|ctx| ctx.process_id.0)
            .unwrap_or(0)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}
