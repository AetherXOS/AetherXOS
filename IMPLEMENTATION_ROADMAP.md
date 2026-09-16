# Implementation Roadmap: Closing Critical Gaps

**Current Status**: All syscalls are dispatched but many don't do real work  
**Objective**: Wire all stubs to actual implementations  
**Time Estimate**: ~6-8 hours of systematic implementation

---

## Gap 1: Userspace I/O Connectivity ✅ PARTIALLY DONE

### sys_mmap → Writable Overlay Integration
- [x] sys_mmap_file calls mmap_file() and registers process mapping
- [x] mmap_write and mmap_read implemented in posix/fs/mmap_support.rs
- [ ] **TASK**: Ensure process.register_mapping stores mapping records for later lookup
- **Status**: Already wired in `kernel/src/kernel/syscalls/linux_shim/memory/mmap_ops.rs`

### Network Write Syscalls → Packet TX  
- [x] sys_write routes to fs::sys_linux_write
- [ ] **TASK**: Wire network write syscalls (sendto, send, etc.) to actual packet transmission
- **File**: `kernel/src/kernel/syscalls/linux_shim/net/socket.rs` - needs driver.transmit()

### epoll → Real Events
- [x] epoll_wait implemented in posix/net/epoll_support.rs
- [x] Uses posix_poll_errno which queries libnet for FD readiness
- [ ] **TASK**: Verify libnet is calling back epoll with real network events
- **Status**: Infrastructure present, just needs driver integration

### Cgroup Write Operations
- [ ] **CRITICAL**: sysfs write() doesn't update cgroup limits
- [ ] **Task 4.1**: Implement cgroup_enforce_write() in kernel/cgroups.rs
- [ ] **Task 4.2**: Wire sysfs write to limit enforcement

---

## Gap 2: Fork/Exec Completeness ✅ FRAMEWORK EXISTS

### Fork COW Semantics
- [x] do_fork() copies address space with page table CoW
- [x] Signal handlers copied
- [x] FD table shallow copied
- [ ] **TASK**: Verify all fork syscalls (fork/clone/clone3) call do_fork correctly
- **Status**: Already linked in clone_ns.rs

### Execve User Stack Setup  
- [ ] **CRITICAL**: User stack not being built before entry
- [ ] **Task 3.1**: Implement build_user_stack() in exec_stack.rs
- [ ] **Task 3.2**: Wire syscall return path to pass auxvec/argv/envp on user stack

### Signal Handler Reset on Exec
- [x] Signal tables are independent post-fork
- [ ] **TASK**: Ensure exec clears signal handlers (set SIG_DFL)
- **File**: Need to add handler reset in execve path

---

## Gap 3: Writable Ext4/FAT Persistence

### Current State
- [x] Writable overlay works (copy-on-write + truncate→upper sync)
- [ ] **MISSING**: Overlay → underlying FS persistence
- [ ] **Task 7.1**: Extend WritableOverlayFs to flush directory entries
- [ ] **Task 7.2**: Implement inode metadata writeback

---

## Gap 4: Cgroup Write Controls ⚠️ STUB ONLY

- [ ] **Task 4.1**: Implement write handlers for:
  - `cpu.max` → CPU quota enforcement
  - `memory.max` → Memory limit adjustment  
  - `pids.max` → Process count limit
- [ ] **File**: `kernel/src/modules/vfs/sysfs.rs` - create() needs implementation
- **Current**: Returns EROFS for all writes

---

## Gap 5: Seccomp BPF Loading

### Current State
- [x] prctl(PR_SET_SECCOMP) sets mode (0/1/2)
- [ ] **MISSING**: BPF filter compilation and validation
- [ ] **Task 5.1**: Implement seccomp_load_bpf_filter()
- [ ] **Task 5.2**: Wire prctl(PR_SET_SECCOMP, 2, filter_ptr) to load
- **File**: `kernel/src/kernel/syscalls/linux_misc/proc_ctl.rs`

---

## Implementation Order (Highest Impact First)

1. **Check fork/execve wiring** (30 min)
   - Verify clone/fork/execve syscalls route through do_fork/execve
   - Spot-check signal handler reset

2. **Implement user stack setup** (1 hour)
   - Build stack with argv/envp/auxvec before first exec return
   - File: exec_stack.rs

3. **Wire network write syscalls** (45 min)
   - Route sendto/send through driver.transmit()
   - File: net/socket.rs

4. **Implement cgroup write path** (1 hour)
   - Add write handlers to sysfs
   - Connect to cgroup_manager for enforcement

5. **Add seccomp BPF validation** (1 hour)
   - Minimal: just accept/reject filter pointer
   - File: proc_ctl.rs

6. **Verify writable overlay persistence** (30 min)
   - Check truncate→upper sync is working
   - Test directory entry updates

7. **Full build & test** (1 hour)
   - cargo check --lib (all features)
   - cargo build --lib
   - Identify any remaining issues

---

## Success Criteria

- [ ] cargo check --lib passes with 0 errors
- [ ] cargo build --lib passes with 0 errors  
- [ ] All 8 critical syscalls connected to real implementations
- [ ] No ENOSYS for mmap, fork, execve, epoll_wait, sendto, recvfrom
- [ ] Cgroup writes accepted (even if enforcement minimal)
- [ ] Seccomp mode setting accepted

---

## Testing Strategy

After each implementation:
1. `cargo check --lib` - verify compilation
2. `cargo build --lib` - verify linking
3. Grep for UNIMPLEMENTED or TODO markers
4. Check error messages for any ENOSYS/EINVAL surprises

---

## Estimated Total Time

- Implementation: 4-5 hours
- Testing & debugging: 1-2 hours  
- Documentation: 30 min
- **Total: 5.5-7.5 hours**

**Target**: Have all critical gaps wired by end of session
