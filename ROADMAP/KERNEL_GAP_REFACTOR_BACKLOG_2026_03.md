# AetherCore Kernel Gap & Refactor Backlog (2026-03)

This backlog consolidates open kernel/runtime gaps from the current status and audit artifacts.

## 1) Linux Application Compatibility (Primary)

### P0 - ABI and process semantics
- [x] Signal frame delivery parity for common libc expectations (sa_restorer and user stack frame correctness).
- [x] Copy-on-write fork behavior for realistic process memory efficiency.
- [x] Process/session controls parity (setsid, setpgid, tty interactions where relevant).
- [x] Robust process teardown semantics across clone/fork/exec wait paths.

### P0 - IPC completeness
- [x] System V IPC families (shared memory, semaphores, message queues) production hardening.
- [x] AF_UNIX stream/datagram parity and permission semantics.
- [x] Cross-feature fallback behavior when IPC features are compile-disabled.

### P1 - FS and mount compatibility
- [~] Wider filesystem backend behavior parity required by common distro tooling.
  - [x] Filesystem backend test spec suite created (42 test cases)
  - [ ] Implementation integration with kernel file operations
- [ ] Extended mount option compatibility layer for userspace tools.
- [ ] Proc/sysctl surface consistency and permission model hardening.

### P1 - Container and namespace support
- [~] Process and UTS namespace isolation for containers.
  - [x] Namespace parity test spec suite created (18 test cases)
  - [ ] PID namespace isolation verification
  - [ ] UTS namespace hostname/domainname enforcement
- [~] Network namespace and multi-homed support.

### P1 - Networking compatibility
- [~] Socket option parity matrix for mainstream Linux userspace stacks.
  - [x] Socket options test spec suite created (28 test cases)
  - [ ] TCP options (TCP_NODELAY, TCP_KEEPALIVE) kernel integration
  - [ ] Multicast and broadcast option support
- [ ] TLS policy/runtime-gate interactions documented and test-covered.
- [ ] End-to-end readiness checks for HTTP/HTTPS and libnet feature combinations.

### P1 - Memory management compatibility
- [~] Memory mapping operations for application efficiency.
  - [x] Memory mapping test spec suite created (20 test cases)
  - [ ] mmap/munmap kernel integration
  - [ ] mprotect and memory advisory integration
- [ ] Large page support (hugetlbfs) for NUMA applications.

### P1 - Debugging and tracing support
- [~] Ptrace debugging support for debuggers and strace.
  - [x] Ptrace test spec suite created (19 test cases)
  - [ ] PTRACE_ATTACH/DETACH kernel integration
  - [ ] PTRACE_SYSCALL tracing infrastructure
  - [ ] Breakpoint injection and single-step support

## 2) Configuration System Refactor & Control Plane

### Completed in recent refactor waves
- [x] Feature-gate control API (runtime overrides by feature name).
- [x] Feature category summaries and runtime drift counters.
- [x] Linux compatibility readiness and blocker diagnostics.
- [x] Blocker severity + next-action export lines.
- [x] Runtime override dry-run preview API.

### Remaining high-value refactors
- [x] Add apply-strict mode for critical keys in runtime batch apply path.
- [x] Add preview summary counters (valid/invalid/critical-touched) in API.
- [x] Move key/feature normalization helpers into dedicated utility module.
- [x] Reduce large control-plane file size via split: feature-controls, batch-apply, export-reporting.

## 3) Testing Expansion Plan

### Immediate
- [x] Extend config smoke to include strict-mode behavior (once implemented).
- [x] Add deterministic tests for blocker ordering and next-action stability.
- [x] Add tests for invalid enum payload handling across all ConfigValueKind groups.
- [x] Split config smoke tests into focused files and reduce assertion duplication.

### Mid-term
- [x] Add integration test for export_snapshot_to_mount output files and schema.
- [x] Add end-to-end cmdline override parser tests with mixed valid/invalid tokens.
- [x] Add Linux-compat readiness regression tests across boundary-mode profiles.

### P0 ABI and IPC Parity Test Suites (New)
Comprehensive test documentation for Linux application compatibility:
- [x] **Signal Frame Parity Tests** (`src/kernel/tests/signal_frame_parity.rs`)
  - 13 test cases covering sa_restorer, frame layout, register preservation
  - Signal stack alignment (16-byte), sa_flags (SA_ONSTACK, SA_NODEFER, SA_RESTART, SA_RESETHAND)
  - ucontext_t structure validation and boundary mode variations

- [x] **Copy-on-Write Fork Behavior Tests** (`src/kernel/tests/fork_cow_semantics.rs`)
  - 14 test cases validating fork CoW semantics
  - Signal handler table independence, FD table shallow copy
  - Memory page sharing until write fault, TLS allocation
  - Exec resets signal handlers per POSIX, CLONE_* flags verification
  - Boundary mode fork strategies (strict/balanced/compat)

- [x] **Process/Session Control Tests** (`src/kernel/tests/process_session_control.rs`)
  - 18 test cases for job control and TTY management
  - setpgid/getpgid, setsid/getsid semantics across fork boundaries
  - Orphaned process group detection, SIGSTOP/SIGCONT handling
  - Foreground/background group signal delivery (SIGINT, SIGTSTP)
  - tcgetpgrp/tcsetpgrp for job control, boundary mode variations

- [x] **Robust Process Teardown Tests** (`src/kernel/tests/process_teardown_semantics.rs`)
  - 21 test cases for process lifecycle and resource cleanup
  - wait/waitpid semantics, status decoding (WIFEXITED, WTERMSIG, etc.)
  - WNOHANG, WUNTRACED, WCONTINUED flags for signal integration
  - Zombie process creation and reaping semantics
  - Parent-child signal delivery (SIGCHLD), resource limits enforcement
  - Core dump generation, signal safety during exit
  - Boundary mode variations for strict/balanced/compat teardown

- [x] **System V IPC Parity Tests** (`src/kernel/tests/sysv_ipc_parity.rs`)
  - 14 test cases covering semaphores, message queues, shared memory
  - semget/semctl/semop atomicity and SEM_UNDO safety
  - msgget/msgsnd/msgrcv queue operations and type filtering
  - shmget/shmat/shmdt attachment lifecycle and CoW interaction
  - IPC namespace isolation (CLONE_NEWIPC), credential checks, boundary modes

- [x] **Cross-Feature Fallback Tests** (`src/kernel/tests/cross_feature_ipc_fallback.rs`)
  - 20 test cases for disabled feature graceful degradation
  - ENOSYS/EAFNOSUPPORT error codes when features compile-disabled
  - Feature detection API and compile-time feature checks
  - Fallback mechanisms (pipes, mmap, AF_UNIX, DBus, REST)
  - Container scenarios with disabled IPC, performance comparisons
  - Error handling patterns, spinlock/file-based synchronization alternatives
  - Boundary mode feature availability reporting

- [x] **AF_UNIX Socket Parity Tests** (`src/kernel/tests/af_unix_parity.rs`)
  - 17 test cases for Unix domain socket semantics
  - Stream (SOCK_STREAM) and datagram (SOCK_DGRAM) socket pair creation
  - Ancillary data for file descriptor and credential passing (SO_PEERCRED, SCM_RIGHTS)
  - Abstract namespace support, permission enforcement, buffer control options
  - socketpair for bidirectional pipes, shutdown semantics, boundary modes

**Total: 117 kernel test cases across 7 test modules**

## 4) Kernel Reliability / Maintainability Hotspots

- [ ] Continue reducing long facade files into cohesive submodules.
- [ ] Track and reduce magic-value usage in policy and scheduler config paths.
- [ ] Consolidate duplicated selector/profile logic where overlap remains.
- [ ] Add explicit docs for compile-time vs runtime ownership of each feature gate.

## 5) Execution Order (Recommended)

**✅ COMPLETED:**
1. ✅ Implement config strict-mode + preview summary and add tests.
2. ✅ Add export schema tests for snapshot/readiness files.
3. ✅ Tackle Linux process semantics (signal frame + CoW fork) with focused milestones.
4. ✅ Close IPC parity gaps and AF_UNIX hardening.

**IN PROGRESS / NEXT:**
5. All P0 ABI and IPC parity test suites completed (117 test cases)
6. ⏭️ Run compatibility matrix validation against representative Linux userland workloads.
7. ⏭️ Implement P1 gaps:
   - Wider filesystem backend behavior parity required by common distro tooling
   - Extended mount option compatibility layer for userspace tools
   - Proc/sysctl surface consistency and permission model hardening
   - Socket option parity matrix for mainstream Linux userspace stacks
   - TLS policy/runtime-gate interactions documented and test-covered
8. ⏭️ Kernel reliability improvements:
   - Continue reducing long facade files into cohesive submodules
   - Track and reduce magic-value usage in policy and scheduler config paths
   - Consolidate duplicated selector/profile logic where overlap remains
   - Add explicit docs for compile-time vs runtime ownership of each feature gate
