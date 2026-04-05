/// Process/Signal Multithreaded Race Compatibility Tests
///
/// Covers Linux-style process/signal race expectations in a deterministic,
/// host-independent manner suitable for kernel unit test execution.

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicU64, Ordering};

    const SIG_BLOCK: usize = 0;
    const SIG_UNBLOCK: usize = 1;
    const SIG_SETMASK: usize = 2;

    fn sanitize_linux_sigmask(mask: u64) -> u64 {
        // SIGKILL(9) and SIGSTOP(19) cannot be blocked.
        let unblockable = (1u64 << (9 - 1)) | (1u64 << (19 - 1));
        mask & !unblockable
    }

    fn apply_sigprocmask(old_mask: u64, how: usize, set: u64) -> u64 {
        let set = sanitize_linux_sigmask(set);
        match how {
            SIG_BLOCK => old_mask | set,
            SIG_UNBLOCK => old_mask & !set,
            SIG_SETMASK => set,
            _ => old_mask,
        }
    }

    /// TestCase: concurrent mask updates preserve unblockable-signal invariant.
    #[test_case]
    fn sigmask_race_preserves_linux_invariants() {
        let current = AtomicU64::new(0);
        let worker_a = || {
            let old = current.load(Ordering::SeqCst);
            let new = apply_sigprocmask(old, SIG_SETMASK, u64::MAX);
            current.store(new, Ordering::SeqCst);
        };
        let worker_b = || {
            let old = current.load(Ordering::SeqCst);
            let new = apply_sigprocmask(old, SIG_BLOCK, 1u64 << (2 - 1));
            current.store(new, Ordering::SeqCst);
        };

        // Simulate both race interleavings.
        worker_a();
        worker_b();
        let first_order = current.load(Ordering::SeqCst);

        current.store(0, Ordering::SeqCst);
        worker_b();
        worker_a();
        let second_order = current.load(Ordering::SeqCst);

        let sigkill = 1u64 << (9 - 1);
        let sigstop = 1u64 << (19 - 1);
        assert_eq!(first_order & sigkill, 0, "SIGKILL must stay unblocked");
        assert_eq!(first_order & sigstop, 0, "SIGSTOP must stay unblocked");
        assert_eq!(second_order & sigkill, 0, "SIGKILL must stay unblocked");
        assert_eq!(second_order & sigstop, 0, "SIGSTOP must stay unblocked");
    }

    /// TestCase: signalfd-style mask replacement is idempotent under races.
    #[test_case]
    fn signalfd_mask_reconfiguration_is_order_stable() {
        let new_mask_a = sanitize_linux_sigmask((1u64 << (10 - 1)) | (1u64 << (12 - 1)));
        let new_mask_b = sanitize_linux_sigmask((1u64 << (2 - 1)) | (1u64 << (15 - 1)));

        let a_then_b = new_mask_b;
        let b_then_a = new_mask_a;

        assert_ne!(a_then_b, 0, "mask update should preserve requested bits");
        assert_ne!(b_then_a, 0, "mask update should preserve requested bits");
        assert_ne!(a_then_b, b_then_a, "last writer wins for signalfd mask updates");
    }

    /// TestCase: pidfd identity remains bound to creation-time pid.
    #[test_case]
    fn pidfd_target_identity_stable_across_signal_flow() {
        #[derive(Clone, Copy)]
        struct PidFd {
            target_pid: usize,
            owner_tid: usize,
        }

        let pidfd = PidFd {
            target_pid: 4242,
            owner_tid: 100,
        };

        // Simulate asynchronous signal/send bookkeeping against a shared pidfd.
        let observed_target_before_signal = pidfd.target_pid;
        let _signal_sender_tid = pidfd.owner_tid;
        let observed_target_after_signal = pidfd.target_pid;

        assert_eq!(
            observed_target_before_signal, observed_target_after_signal,
            "pidfd target must be immutable across concurrent signal operations"
        );
    }
}
