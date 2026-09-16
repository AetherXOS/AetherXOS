# 11 Subsystems to Completion (11/11 Sprint)

## Overall Goal
Achieve production-readiness for **all 11 core subsystems** by:
- Closing critical implementation gaps
- Integrating P0/P1 test coverage (117 test cases)
- Stabilizing weak areas (TTY, signals, process model)
- Ensuring all subsystems build + tests pass

**Current Status:** ~45-50% overall, P0: 80%, P1: 55%, P2: 20%

---

## 11 Subsystems Status Matrix

| # | Subsystem | Completion | Priority | Key Issues | Owner |
|---|-----------|-----------|----------|------------|-------|
| 1 | **Memory Management** | 70% ✅ | Medium | Page cache edge cases | Hardware HAL |
| 2 | **Scheduler (CFS+RT)** | 60% ⚠️ | High | Stalls/hangs on overload | Scheduler |
| 3 | **Process Model** | 50% ⚠️ | **High** | Fork/CoW gaps, wait/reaping incomplete | Process |
| 4 | **IPC System** | 45% ⚠️ | **High** | System V incomplete, AF_UNIX partial | IPC |
| 5 | **Security/Governance** | ~50% ⚠️ | Medium | Resource limits, cgroup enforcement | Security |
| 6 | **VFS/Filesystem** | 65% ✅ | Low | xattr edge cases, mount options | VFS |
| 7 | **Network Stack** | 55% ⚠️ | High | Socket options incomplete, epoll edge cases | Network |
| 8 | **Linux Compat Shim** | 60% ⚠️ | **Critical** | syscall semantic parity, errno handling | Linux Compat |
| 9 | **POSIX Abstraction Layer** | 55% ⚠️ | High | namespace isolation, exec behavior | POSIX |
| 10 | **Device Drivers** | ~40% ⚠️ | Medium | VirtIO edge cases, interrupt handling | Drivers |
| 11 | **Dispatcher/HAL** | 50% ⚠️ | Medium | CPU affinity edge cases, fault routing | Dispatcher |

---

## Critical Gaps (Blocking 11/11)

### Tier 1: BLOCKING (Complete First)

#### 1. **TTY/Job Control** (15% → Target: 80%)
Currently a near-total stub. This blocks interactive shell support.

**What's missing:**
- [ ] TTY device model (tty_struct equivalent)
- [ ] Job control signals (SIGTSTP, SIGCONT handling)
- [ ] Process group/session lifecycle (setpgrp, setsid)
- [ ] Terminal I/O control (tcsetattr, tcgetattr)
- [ ] Signal delivery to process groups
- [ ] TTY line discipline integration

**Key files to create/enhance:**
- `src/kernel/tty/mod.rs` (new)
- `src/kernel/tty/job_control.rs` (new)
- `src/kernel/signal/group_delivery.rs` (new - extend signal handling)
- Update: `src/modules/linux_compat/sys_dispatcher/process.rs`

**Test cases:** P0 module "Process/Session Control" (18 tests)

---

#### 2. **Signal Frame Delivery** (40% → Target: 85%)
Libc expects proper signal frame layout for restorers and unwinding.

**What's missing:**
- [ ] Complete sa_restorer callback support (currently ~20%)
- [ ] Proper sigreturn frame layout (ia64/aarch64)
- [ ] Signal stack (SS_ONSTACK) handling
- [ ] Nested signal delivery (mask stacking)
- [ ] Real-time signal queuing (SIGRTMIN..SIGRTMAX)
- [ ] SIGEV_THREAD callback support

**Key files to enhance:**
- `src/kernel/signal/frame.rs` (expand)
- `src/kernel/signal/restorer.rs` (new - sa_restorer support)
- `src/kernel/signal/rt_queues.rs` (new - real-time signal queue)

**Test cases:** P0 module "Signal Frame Parity" (13 tests)

---

#### 3. **Process Model Gaps** (50% → Target: 85%)
Fork/wait semantics incomplete, affecting multi-process apps.

**What's missing:**
- [ ] Full Copy-on-Write implementation (currently partial)
- [ ] Process/task tree management (parent tracking, zombie handling)
- [ ] wait/waitpid/waitid complete semantics
- [ ] WCONTINUED/WUNTRACED flag support
- [ ] Process exit status encoding (WIFEXITED, etc.)
- [ ] Ptrace integration for debugging

**Key files to enhance:**
- `src/kernel/process/fork.rs` (expand CoW)
- `src/kernel/process/wait.rs` (complete wait logic)
- `src/kernel/process/ptrace.rs` (new/expand)

**Test cases:** P0 modules "Fork CoW Semantics" (14), "Process Teardown" (21)

---

#### 4. **IPC System Completion** (45% → Target: 80%)
System V IPC and AF_UNIX need full behavioral parity with Linux.

**What's missing:**
- [ ] System V semaphores: semctl full opcodes (IPC_SET, IPC_STAT, GETNCNT, etc.)
- [ ] System V message queues: msgsnd/msgrcv priority queuing
- [ ] System V shared memory: mmap-like shm semantics
- [ ] AF_UNIX datagram: connectionless semantics
- [ ] Unix socket ancillary data (SCM_CREDENTIALS, SCM_FDS)
- [ ] Socket pair (socketpair) full implementation

**Key files to enhance:**
- `src/modules/ipc/sysv_sem.rs` (expand)
- `src/modules/ipc/sysv_msg.rs` (expand)
- `src/modules/ipc/sysv_shm.rs` (expand)
- `src/modules/posix/sockets/unix.rs` (expand)

**Test cases:** P0 modules "System V IPC" (14), "AF_UNIX Sockets" (17)

---

### Tier 2: HIGH Impact (Complete Second)

#### 5. **Linux Compat Semantic Parity** (60% → Target: 90%)
Syscall semantics must match Linux exactly for app compatibility.

**What's missing:**
- [ ] errno conformance matrix (EFAULT vs EACCES, etc.) - partially done
- [ ] Signal mask restoration on error paths (pselect6, ppoll)
- [ ] Timeout rounding edge cases (poll, select, epoll_wait)
- [ ] Descriptor validation and out-of-bounds handling
- [ ] File descriptor limits (RLIMIT_NOFILE enforcement)
- [ ] Cross-feature fallback chains (e.g., epoll → poll → select)

**Key files to enhance:**
- `src/modules/linux_compat/net/poll.rs` (expand)
- `src/modules/linux_compat/sys_dispatcher/*.rs` (semantic hardening)
- Add: `src/modules/linux_compat/errno_test_matrix.rs` (test)

**Test cases:** P0 module "Cross-Feature Fallback" (20 tests)

---

#### 6. **Socket Options & Advanced Features** (35% → Target: 70%)
SO_REUSEADDR works, but TCP_* options, multicast, etc. missing.

**What's missing:**
- [ ] TCP_NODELAY, TCP_KEEPALIVE, TCP_CORK
- [ ] SO_KEEPALIVE, SO_LINGER, SO_RCVBUF, SO_SNDBUF
- [ ] Multicast join/leave (IP_ADD_MEMBERSHIP, IPV6_JOIN_GROUP)
- [ ] IPTOS_* / IP_TOS routing priority
- [ ] TCP FastOpen (TCP_FASTOPEN)
- [ ] UDP_SEGMENT (UDP GSO)

**Key files to enhance:**
- `src/modules/linux_compat/net/socket.rs` (expand getsockopt/setsockopt)
- New: `src/modules/linux_compat/net/socket_opts.rs`

**Test cases:** P1 module "Socket Options" (28 tests)

---

### Tier 3: MEDIUM Impact (Complete Third)

#### 7. **Namespace Isolation** (25% → Target: 60%)
PID/UTS works basic, but Mount/Network/IPC namespaces incomplete.

**What's missing:**
- [ ] Mount namespace proper isolation (pivot_root semantics)
- [ ] PID namespace proc visibility
- [ ] Network namespace (veth, bridge isolation, route table per NS)
- [ ] IPC namespace (sys V objects isolation)
- [ ] User namespace (uid mapping)
- [ ] Namespace lifecycle (unshare, setns edge cases)

**Key files to enhance:**
- `src/modules/posix/namespaces/mod.rs` (orchestrate)
- `src/modules/posix/namespaces/mount.rs` (expand)
- `src/modules/posix/namespaces/network.rs` (new/expand)
- `src/modules/posix/namespaces/pid.rs` (expand)

**Test cases:** P1 module "PID/UTS Namespace" (18 tests)

---

#### 8. **Device Driver Hardening** (40% → Target: 75%)
VirtIO works basic, but edge cases and interrupt handling weak.

**What's missing:**
- [ ] VirtIO error recovery (queue stall handling)
- [ ] MSI-X interrupt routing per-queue
- [ ] Device hotplug/removal
- [ ] E1000 legacy driver support
- [ ] Device reset semantics (DEVICE_NEEDS_RESET)
- [ ] Interrupt coalescing / adaptive polling

**Key files to enhance:**
- `src/modules/drivers/virtio_*/queue.rs` (hardening)
- `src/modules/drivers/interrupt_handler.rs` (expand)

---

## Test Integration Plan (117 Cases)

### P0 (Critical ABI - 7 modules, 117 tests)

1. **Signal Frame Parity** (13 tests)
   - sa_restorer correctness
   - Frame layout per-architecture
   - Signal stack boundaries

2. **Fork CoW Behavior** (14 tests)
   - Memory page sharing
   - Concurrent modification detection
   - Page reclaim on divergence

3. **Process/Session Control** (18 tests) ⚠️ **Priority**
   - setpgrp/setsid/getpgrp
   - Job control signals
   - TTY attachment

4. **Process Teardown** (21 tests) ⚠️ **Priority**
   - wait/waitpid semantics
   - Zombie process handling
   - Exit status encoding

5. **System V IPC** (14 tests)
   - semctl full opcodes
   - msgsnd/msgrcv with priority
   - shmat/shmdt with flags

6. **Cross-Feature Fallback** (20 tests)
   - ENOSYS → fallback chain
   - errno correctness per path
   - Timeout handling consistency

7. **AF_UNIX Sockets** (17 tests)
   - Stream and datagram modes
   - Ancillary data (SCM_CREDENTIALS)
   - Socket pair behavior

---

## Implementation Roadmap (Week-Based)

### Week 1: TTY & Signal (Tier 1 - Part A)
**Deliverables:**
- TTY device model (tty_struct in kernel/tty/mod.rs)
- Job control signal delivery (SIGTSTP, SIGCONT)
- Process group/session lifecycle (setpgrp, setsid)
- Signal restorer support (sa_restorer callbacks)
- **Pass:** 31 test cases (Process/Session Control 18 + Signal Frame 13)

**Related Files (Create/Modify):**
```
CREATE: src/kernel/tty/mod.rs
CREATE: src/kernel/tty/job_control.rs
CREATE: src/kernel/signal/restorer.rs
CREATE: src/kernel/signal/group_delivery.rs
MODIFY: src/modules/linux_compat/sys_dispatcher/process.rs
MODIFY: src/kernel/signal/frame.rs
```

---

### Week 2: Process Model & Wait (Tier 1 - Part B)
**Deliverables:**
- Full Copy-on-Write in fork path
- wait/waitpid/waitid complete semantics
- Process tree + zombie handling
- Exit status encoding helpers
- **Pass:** 35 test cases (Fork CoW 14 + Process Teardown 21)

**Related Files:**
```
MODIFY: src/kernel/process/fork.rs (expand CoW)
MODIFY: src/kernel/process/wait.rs (complete logic)
CREATE: src/kernel/process/exit_status.rs
MODIFY: src/kernel/process/ptrace.rs (expand)
```

---

### Week 3: IPC System (Tier 1 - Part C)
**Deliverables:**
- System V semaphores: full semctl opcodes
- System V message queues: priority queuing
- System V shared memory: attach/detach semantics
- AF_UNIX: datagram + ancillary data
- **Pass:** 31 test cases (System V IPC 14 + AF_UNIX 17)

**Related Files:**
```
MODIFY: src/modules/ipc/sysv_sem.rs (expand)
MODIFY: src/modules/ipc/sysv_msg.rs (expand)
MODIFY: src/modules/ipc/sysv_shm.rs (expand)
MODIFY: src/modules/posix/sockets/unix.rs (expand)
CREATE: src/modules/ipc/sysv_helpers.rs
```

---

### Week 4: Linux Compat Hardening (Tier 2 - Part A)
**Deliverables:**
- errno conformance matrix (EFAULT vs EACCES)
- Signal mask restoration on error
- Timeout rounding edge cases
- Descriptor validation + limits
- Cross-feature fallback chains
- **Pass:** 20 test cases (Cross-Feature Fallback 20)

**Related Files:**
```
MODIFY: src/modules/linux_compat/net/poll.rs
MODIFY: src/modules/linux_compat/sys_dispatcher/*.rs
CREATE: src/modules/linux_compat/errno_matrix.rs
CREATE: scripts/linux_errno_conformance_test.py
```

---

### Week 5: Socket Options & Namespaces (Tier 2 - Part B + Tier 3 - Part A)
**Deliverables:**
- TCP/UDP socket options (TCP_NODELAY, SO_KEEPALIVE, etc.)
- Multicast support (IP_ADD_MEMBERSHIP)
- Socket options matrix (28 tests)
- Mount namespace isolation + pivot_root
- PID namespace proc visibility
- **Pass:** 46 test cases (Socket Options 28 + Namespaces 18)

**Related Files:**
```
MODIFY: src/modules/linux_compat/net/socket.rs
CREATE: src/modules/linux_compat/net/socket_opts.rs
MODIFY: src/modules/posix/namespaces/mount.rs
MODIFY: src/modules/posix/namespaces/pid.rs
CREATE: src/modules/posix/namespaces/network.rs
```

---

### Week 6: Consolidation & Verification (Tier 3)
**Deliverables:**
- Device driver hardening (VirtIO error recovery, MSI-X routing)
- Network namespace completion
- All 117 P0 tests integrated + passing
- Build verification: `cargo check -q` + all subsystem tests
- Code quality audit (dead code cleanup, monolithic file splits if needed)

**Related Files:**
```
MODIFY: src/modules/drivers/virtio_*/queue.rs
MODIFY: src/modules/drivers/interrupt_handler.rs
MODIFY: src/modules/posix/namespaces/network.rs
RUN: Integrate all 117 test cases
RUN: cargo test --no-run (validate compiles)
```

---

## Success Criteria

### Build & Compilation
- ✅ `cargo check -q` **PASSES** (0 errors)
- ✅ All subsystem features compile independently
- ✅ All 117 P0 tests compile (no integration failures)

### Functionality
- ✅ TTY/Job control: 18 tests passing
- ✅ Signal handling: 13+20 tests passing
- ✅ Process model: 14+21 tests passing
- ✅ IPC system: 14+17 tests passing
- ✅ Linux compat: 20 tests passing
- ✅ Socket options: 28 tests passing
- ✅ Namespaces: 18 tests passing

### Code Quality
- ✅ No monolithic files > 2000 LOC (refactor if needed)
- ✅ All #[allow(dead_code)] justified or removed
- ✅ Feature flag nesting < 3 levels deep
- ✅ Syscall semantic conformance matrix completed

### Stability
- ✅ No scheduler hangs/stalls under load
- ✅ Multi-process coordination stable
- ✅ Signal delivery reliable (no lost signals)
- ✅ IPC operation atomic/consistent

---

## Blockers & Risks

| Issue | Impact | Mitigation |
|-------|--------|-----------|
| Scheduler stalls under heavy fork load | HIGH | Profile + fix task queue bounds |
| Signal mask restoration complexity | HIGH | Add comprehensive state machine tests |
| Cross-arch (x86/aarch64) signal frame layout | HIGH | Test both paths in CI |
| System V IPC permission model gaps | MEDIUM | Reference Linux semget/msgget behavior |
| Mount namespace pivot_root edge cases | MEDIUM | Add chroot/pivot_root tests |

---

## Next Steps

**START HERE (This Sprint):**
1. ✅ Accept this plan
2. → Create TTY device model skeleton (Week 1)
3. → Implement job control signal delivery (Week 1)
4. → Integrate Process/Session Control tests (18 tests)
5. → Verify build still green

**Expected Outcome After This Sprint:**
- **11 subsystems at 65-75% completeness**
- **117 P0 tests integrated + 60+ passing**
- **TTY/Signal/Process model no longer blockers**

