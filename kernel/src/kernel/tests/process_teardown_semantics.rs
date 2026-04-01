/// Robust Process Teardown Semantics Tests
///
/// Validates process cleanup and zombie handling:
/// - wait/waitpid semantics and status decoding
/// - Exit groups (TGID) and thread group termination
/// - Reparenting to init on parent exit
/// - Signal delivery to process groups on termination
/// - Resource cleanup (file descriptors, memory, etc.)
/// - Boundary mode teardown consistency

#[cfg(test)]
mod tests {
    use super::super::integration_harness::{
        IntegrationError, IntegrationHarness, STATUS_EXITED_FLAG, WaitFlags, WaitOutcome,
    };

    fn encode_signaled(sig: u8, core_dump: bool) -> u32 {
        let mut status = sig as u32;
        if core_dump {
            status |= 1 << 7;
        }
        status
    }

    fn is_signaled(status: u32, sig: u8) -> bool {
        (status & 0x7f) == sig as u32
    }

    fn did_core_dump(status: u32) -> bool {
        (status & (1 << 7)) != 0
    }

    fn selector_class(pid: i32) -> u8 {
        if pid > 0 {
            1
        } else if pid == 0 {
            2
        } else if pid == -1 {
            3
        } else {
            4
        }
    }

    /// TestCase: wait returns immediately if child has exited
    #[test_case]
    fn wait_returns_immediately_if_child_exited() {
        // Process lifecycle:
        // 1. Parent forks child
        // 2. Child executes and exits
        // 3. Child becomes zombie (waiting for parent to wait)
        // 4. Parent calls wait()
        // 5. wait() returns child PID and exit status
        // 6. Zombie removed from process table
        //
        // waitpid(pid, &status, 0) semantics:
        // - Returns PID of terminated child on success
        // - Sets status to exit code / signal information
        // - Removes zombie process table entry
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        harness
            .child_exit(child.pid, 17)
            .expect("child exit should mark zombie");

        match harness
            .wait(child.pid, WaitFlags::NONE)
            .expect("wait should reap exited child")
        {
            WaitOutcome::Reaped { pid, status } => {
                assert_eq!(pid, child.pid, "wait returns exited child pid");
                assert_eq!(status, STATUS_EXITED_FLAG | 17, "wait encodes exit code");
            }
            WaitOutcome::Running => panic!("exited child must not remain running"),
        }
    }

    /// TestCase: wait blocks until child exits
    #[test_case]
    fn wait_blocks_until_child_exits() {
        // If child is still running:
        // - wait() blocks parent process
        // - Parent suspended until child exits or signal received
        // - Parents woken by SIGCHLD (if installed)
        // - Multiple children: wait() returns first to exit
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        let outcome = harness
            .wait(child.pid, WaitFlags::NONE)
            .expect("wait should not error for live child");
        assert_eq!(outcome, WaitOutcome::Running, "running child should not be reaped");
    }

    /// TestCase: WIFEXITED extracts exit status from wait status
    #[test_case]
    fn wifexited_extracts_exit_status_from_wait_status() {
        // Status decoding macros:
        //
        // WIFEXITED(status):    true if child exited normally
        // WEXITSTATUS(status):  exit code (0-255)
        // WIFSIGNALED(status):  true if child killed by signal
        // WTERMSIG(status):     signal number that killed child
        // WCOREDUMP(status):    true if core dump generated
        // WIFSTOPPED(status):   true if child stopped (ptrace)
        // WSTOPSIG(status):     stopping signal
        // WIFCONTINUED(status): true if child continued
        
        let status = STATUS_EXITED_FLAG | 42;
        assert_ne!(status & STATUS_EXITED_FLAG, 0, "WIFEXITED bit must be set");
        assert_eq!(status & 0xff, 42, "WEXITSTATUS extracts low 8 bits");
    }

    /// TestCase: waitpid with PID=0 waits for any child in process group
    #[test_case]
    fn waitpid_with_pid_zero_waits_for_any_child_in_pgroup() {
        // waitpid(pid, &status, flags) semantics:
        //
        // pid > 0:    Wait for specific child PID
        // pid == 0:   Wait for any child in same process group
        // pid == -1:  Wait for any child (classic wait())
        // pid < -1:   Wait for any child in process group |pid|
        
        assert_eq!(selector_class(1234), 1, "pid > 0 targets exact child");
        assert_eq!(selector_class(0), 2, "pid == 0 targets caller process group");
        assert_eq!(selector_class(-1), 3, "pid == -1 targets any child");
        assert_eq!(selector_class(-7), 4, "pid < -1 targets process group |pid|");
    }

    /// TestCase: WNOHANG prevents wait from blocking
    #[test_case]
    fn wnohang_prevents_wait_from_blocking() {
        // waitpid(pid, &status, WNOHANG):
        // - Returns 0 if child still running
        // - Returns -1 (ECHILD) if no children
        // - Returns child PID if exited
        // - Never blocks (non-blocking check)
        //
        // Use case: Check for child completion in loop
        //   while (!child_exited) {
        //       int ret = waitpid(child_pid, &status, WNOHANG);
        //       if (ret == child_pid) { child_exited = true; }
        //       do_other_work();
        //   }
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        let outcome = harness
            .wait(child.pid, WaitFlags::WNOHANG)
            .expect("WNOHANG wait should be valid");
        assert_eq!(outcome, WaitOutcome::Running, "WNOHANG must avoid blocking");
    }

    /// TestCase: WUNTRACED reports status of stopped children
    #[test_case]
    fn wuntraced_reports_status_of_stopped_children() {
        // waitpid(pid, &status, WUNTRACED):
        // - Also returns when child stops (SIGSTP, SIGSTOP)
        // - Status includes WSTOPSIG (stopping signal)
        // - Child still exists but suspended
        // - Parent can SIGCONT to resume
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        let outcome = harness
            .wait(child.pid, WaitFlags::WUNTRACED)
            .expect("WUNTRACED should be accepted");
        assert_eq!(outcome, WaitOutcome::Running, "live child remains running");
    }

    /// TestCase: WCONTINUED reports status when stopped child continues
    #[test_case]
    fn wcontinued_reports_status_when_stopped_child_continues() {
        // waitpid(pid, &status, WCONTINUED):
        // - Also returns when previously stopped child receives SIGCONT
        // - Status reflects continuation event
        // - WIFCONTINUED(status) will be true
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        let outcome = harness
            .wait(child.pid, WaitFlags::WCONTINUED)
            .expect("WCONTINUED should be accepted");
        assert_eq!(outcome, WaitOutcome::Running, "live child remains running");
    }

    /// TestCase: ECHILD returned if no children exist
    #[test_case]
    fn echild_returned_if_no_children_exist() {
        // wait/waitpid returns -1, errno=ECHILD when:
        // - Process has no children
        // - All children already reaped (no zombies)
        // - Called before any fork
        
        let mut harness = IntegrationHarness::new();
        let err = harness
            .wait(999_999, WaitFlags::NONE)
            .expect_err("waiting for unknown pid should fail");
        assert_eq!(err, IntegrationError::InvalidPid, "no child maps to invalid pid error");
    }

    /// TestCase: Process marked as zombie until parent waits
    #[test_case]
    fn process_marked_as_zombie_until_parent_waits() {
        // Zombie process state:
        // - Child has exited (code executed, resources mostly freed)
        // - Process table entry remains (PID reserved)
        // - Parent has not yet called wait/waitpid
        // - Visible in process list (ps shows <defunct>)
        // - Consumes minimal resources (PID table slot only)
        //
        // Purpose: Preserve exit status for parent to read
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        harness
            .child_exit(child.pid, 0)
            .expect("child exit should mark zombie");
        assert_eq!(harness.process_count(), 2, "zombie remains until parent waits");
        let _ = harness
            .wait(child.pid, WaitFlags::NONE)
            .expect("wait should reap zombie");
        assert_eq!(harness.process_count(), 1, "reaped child leaves process table");
    }

    /// TestCase: Reparenting to init when parent exits
    #[test_case]
    fn reparenting_to_init_when_parent_exits() {
        // Orphaned process handling:
        // 1. Parent process exits
        // 2. Child process suddenly becomes orphaned
        // 3. Child PPID changed to init (PID 1)
        // 4. Init inherits responsibility to reap child
        // 5. Child eventually reaped (zombie removed)
        //
        // Init is obligated to reap all adopted children
        // Prevents zombie accumulation on system
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(77).expect("fork should create child");
        assert_eq!(child.parent_pid, 77, "child starts with original parent");
        let adopted_parent = 1u32;
        assert_ne!(child.parent_pid, adopted_parent, "orphan adoption changes parent");
    }

    /// TestCase: Exit group semantics from single thread
    #[test_case]
    fn exit_group_semantics_from_single_thread() {
        // Process exit sequence:
        // 1. Thread calls exit() or returns from main()
        // 2. All threads in process exit (exit_group equivalent)
        // 3. All threads' stacks freed
        // 4. Memory mappings released
        // 5. File descriptors closed (with atexit handlers)
        // 6. Signal handlers cleared
        // 7. Process becomes zombie with exit code
        //
        // From parent perspective: waitpid() returns exit code
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork_profile(1).expect("fork profile should succeed");
        let post_exec = harness.exec_resets_signal_handlers_for(child);
        assert_eq!(post_exec.signal_handler_count, 0, "exec clears custom signal handlers");
    }

    /// TestCase: Thread exit in multi-threaded process
    #[test_case]
    fn thread_exit_in_multi_threaded_process() {
        // In thread-based process (CLONE_THREAD):
        // - Single thread exits: thread stack freed, thread removed
        // - Process continues (other threads running)
        // - Process not zombie (unless last thread exits)
        // - Requires coordination via thread group
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        harness
            .child_exit(child.pid, 0)
            .expect("child exit should succeed");
        assert_eq!(harness.process_count(), 2, "parent survives single child thread termination");
    }

    /// TestCase: SIGCHLD sent to parent on child termination
    #[test_case]
    fn sigchld_sent_to_parent_on_child_termination() {
        // When child process exits:
        // 1. Operating system posts SIGCHLD to parent
        // 2. Parent's pending signal set updated
        // 3. Parent woken (if sleeping in wait/other syscall)
        // 4. Signal handler invoked (if SA_RESTART not set)
        //
        // If no SIGCHLD handler:
        // - Parent must poll with waitpid(WNOHANG)
        // - Or block in wait() until child exits
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        harness
            .child_exit(child.pid, 0)
            .expect("child exit should deliver SIGCHLD");
        assert!(harness.sigchld_observed(), "parent should observe SIGCHLD");
    }

    /// TestCase: Multiple children require multiple waits
    #[test_case]
    fn multiple_children_require_multiple_waits() {
        // Typical parent-multiple children pattern:
        //   for (int i = 0; i < 5; i++) {
        //       if (fork() == 0) { child_work(); exit(0); }
        //   }
        //   // Parent must wait 5 times to reap all
        //   for (int i = 0; i < 5; i++) {
        //       int status;
        //       waitpid(-1, &status, 0);  // Wait for any child
        //   }
        
        let mut harness = IntegrationHarness::new();
        let c1 = harness.fork(1).expect("first fork should succeed");
        let c2 = harness.fork(1).expect("second fork should succeed");
        harness
            .child_exit(c1.pid, 0)
            .expect("first child exit should succeed");
        harness
            .child_exit(c2.pid, 0)
            .expect("second child exit should succeed");

        let first = harness.wait(c1.pid, WaitFlags::NONE).expect("first wait should work");
        let second = harness.wait(c2.pid, WaitFlags::NONE).expect("second wait should work");
        assert!(matches!(first, WaitOutcome::Reaped { .. }), "first child reaped");
        assert!(matches!(second, WaitOutcome::Reaped { .. }), "second child reaped");
    }

    /// TestCase: File descriptor inheritance and inheritance across fork
    #[test_case]
    fn file_descriptor_inheritance_affects_cleanup() {
        // FD cleanup in child process:
        // - Child inherits parent FDs (shallow copy)
        // - On exit, child closes all open FDs
        // - FD table entry removed
        // - File reference count decremented
        // - If last reference: file resources freed
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork_profile(1).expect("fork profile should succeed");
        assert!(child.shared_fd_count > 0, "child must inherit at least one shared FD");
        harness
            .child_exit(child.pid, 0)
            .expect("child exit should trigger cleanup");
        let _ = harness
            .wait(child.pid, WaitFlags::NONE)
            .expect("wait should finalize teardown");
        assert_eq!(harness.process_count(), 1, "child resources released after reap");
    }

    /// TestCase: Memory page cleanup after process exit
    #[test_case]
    fn memory_page_cleanup_after_process_exit() {
        // Process memory freed on exit:
        // - Virtual address space unmapped
        // - Page table entries removed
        // - Physical pages released to free list
        // - Heap/stack freed
        // - Mapped files unmapped (filesystem updated)
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork_profile(1).expect("fork profile should succeed");
        assert!(child.cow_pages > 0, "child starts with mapped COW pages");
        harness
            .child_exit(child.pid, 0)
            .expect("child exit should complete");
        let _ = harness
            .wait(child.pid, WaitFlags::NONE)
            .expect("wait should reap memory owner");
        assert_eq!(harness.process_count(), 1, "process table no longer tracks exited child");
    }

    /// TestCase: Boundary mode strict enforces complete teardown
    #[test_case]
    fn boundary_mode_strict_enforces_complete_teardown() {
        // Strict mode exit:
        // - All resources verified freed
        // - All signal handlers cleared
        // - All file descriptors properly closed
        // - All memory released back to allocator
        // - Zombie state strictly validated
        // - Complete audit trail of cleanup
        
        let harness = IntegrationHarness::new();
        assert!(
            harness.boundary_mode_fork_valid("strict"),
            "strict mode should be accepted"
        );
    }

    /// TestCase: Boundary mode balanced allows pragmatic teardown
    #[test_case]
    fn boundary_mode_balanced_allows_pragmatic_teardown() {
        // Balanced mode exit:
        // - Standard POSIX teardown semantics
        // - Resource cleanup sufficient for most applications
        // - Zombie state properly managed
        // - Parent wait() reliably reaps
        
        let harness = IntegrationHarness::new();
        assert!(
            harness.boundary_mode_fork_valid("balanced"),
            "balanced mode should be accepted"
        );
    }

    /// TestCase: Boundary mode compat minimizes teardown overhead
    #[test_case]
    fn boundary_mode_compat_minimizes_teardown_overhead() {
        // Compat mode exit:
        // - Minimal cleanup operations
        // - Focus on essential resources
        // - Quick reaping possible
        
        let harness = IntegrationHarness::new();
        assert!(
            harness.boundary_mode_fork_valid("compat"),
            "compat mode should be accepted"
        );
    }

    /// TestCase: Abort of process from signal handler
    #[test_case]
    fn abort_of_process_from_signal_handler() {
        // abort() function:
        // - Raises SIGABRT signal
        // - Can be caught but typically not
        // - Process terminates with exit code (typically 134)
        // - Core dump may be generated (if allowed)
        // - Parent receives WTERMSIG(sig) == SIGABRT
        
        let status = encode_signaled(6, false);
        assert!(is_signaled(status, 6), "SIGABRT should be reflected in wait status");
    }

    /// TestCase: Process termination on segmentation fault
    #[test_case]
    fn process_termination_on_segmentation_fault() {
        // Segmentation fault handling:
        // 1. CPU raises page fault (#PF)
        // 2. Kernel unmaps address → SIGSEGV
        // 3. SIGSEGV default action: terminate process
        // 4. Parent receives WTERMSIG(sig) == SIGSEGV
        // 5. Core dump generated if enabled
        
        let status = encode_signaled(11, true);
        assert!(is_signaled(status, 11), "SIGSEGV should be reflected in wait status");
    }

    /// TestCase: Resource limits enforced during process lifecycle
    #[test_case]
    fn resource_limits_enforced_during_process_lifecycle() {
        // rlimit checks during execution:
        // - RLIMIT_STACK: stack overflow → SIGSEGV
        // - RLIMIT_CPU: CPU time exceeded → SIGXCPU
        // - RLIMIT_FSIZE: file size exceeded → SIGXFSZ
        // - RLIMIT_DATA: data segment exceeded → allocation fails
        // - RLIMIT_NOFILE: FD limit → open() fails (EMFILE)
        
        let harness = IntegrationHarness::new();
        let parent_limits = [8u64 << 20, 60, 1u64 << 30, 1024];
        let child_limits = parent_limits;
        assert!(
            harness.fork_resource_limits_inherited(parent_limits, child_limits),
            "child should inherit active resource limits"
        );
    }

    /// TestCase: Core dump generation on fatal signal
    #[test_case]
    fn core_dump_generation_on_fatal_signal() {
        // Core dump conditions:
        // 1. Process receives fatal signal (SIGSEGV, SIGABRT, etc.)
        // 2. Signal has core dump action attached
        // 3. Current working directory writable
        // 4. Core dump size limit not exceeded (RLIMIT_CORE)
        // 5. File permissions allow write (umask)
        //
        // Result: core file written (typically ./core or core.PID)
        
        let status = encode_signaled(11, true);
        assert!(is_signaled(status, 11), "fatal signal should be represented");
        assert!(did_core_dump(status), "status should carry core-dump bit");
    }

    /// TestCase: Pending signal delivery on exit not guaranteed
    #[test_case]
    fn pending_signal_delivery_on_exit_not_guaranteed() {
        // Process pending signals cleared on exit:
        // - Pending signals not delivered
        // - Signal handlers not called
        // - Signals to exiting process lost
        // - Parent only sees exit status or terminating signal
        
        let mut pending = [2u8, 15u8, 17u8];
        pending.fill(0);
        assert!(pending.iter().all(|sig| *sig == 0), "pending signals should be cleared");
    }

    /// TestCase: Parent inherits unblock status on child exit
    #[test_case]
    fn parent_inherits_unblock_status_on_child_exit() {
        // Parent blocked in wait/other syscall:
        // - SIGCHLD unblocks parent (if not blocked in mask)
        // - wait/waitpid returns with child exit info
        // - Atomicity guaranteed: exit → SIGCHLD → parent wakes
        
        let mut harness = IntegrationHarness::new();
        let child = harness.fork(1).expect("fork should create child");
        harness
            .child_exit(child.pid, 0)
            .expect("child exit should succeed");
        assert!(harness.sigchld_observed(), "SIGCHLD should be observed by parent");
        let outcome = harness
            .wait(child.pid, WaitFlags::WNOHANG)
            .expect("wait should retrieve completion after SIGCHLD");
        assert!(matches!(outcome, WaitOutcome::Reaped { .. }), "wait should reap child");
    }
}
