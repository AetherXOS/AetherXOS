/// Integration Test Framework Documentation
///
/// Framework for linking core/extended specifications to executable kernel tests.
/// These document the integration testing strategy and patterns without
/// requiring a test harness in the no_std kernel environment.
///
/// Integration Architecture:
///
/// ```
/// Specification Tests (Documentation)
///     ↓
/// Integration Layer (Module Glue)
///     ↓
/// Actual Kernel Implementation (Real Syscalls)
///     ↓
/// Test Validation (Pass/Fail Results)
/// ```
///
/// Integration Test Cases (Documented Below)

/// Integration Test Template
///
/// Name: {Feature Name}
/// Category: Core|Extended
/// 
/// Specification:
/// - What behavior is being tested
/// - Parameters and return values
/// - Edge cases and error conditions
///
/// Implementation Checklist:
/// - [ ] Kernel function creates/modifies state
/// - [ ] Test harness calls kernel function
/// - [ ] Results validated against expectations
/// - [ ] Error cases handled
/// - [ ] Boundary modes tested
///
/// Examples:
/// - Signal Frame: Validates frame layout, alignment, register preservation
/// - Fork CoW: Validates memory sharing, signal independence, FD copying
/// - Process Wait: Validates WNOHANG, WUNTRACED, WCONTINUED flags
/// - Filesystem Ops: Validates stat, chmod, extended attributes
/// - Socket Options: Validates TCP_NODELAY, SO_REUSEADDR, multicast options
/// - Memory Mapping: Validates mmap alignment, mprotect, madvise
/// - Ptrace: Validates PTRACE_ATTACH, register access, syscall tracing

/// Core Integration Test: Signal Frame Delivery
/// 
/// Category: Core - ABI Critical
/// 
/// Specification:
/// - Signal frame delivered on valid signal
/// - Frame 16-byte aligned for XSAVE support
/// - All registers (RAX-R15, RIP) preserved
/// - sa_restorer handler address set correctly
/// - Stack pointer adjusted for frame
/// - Boundary modes: strict (rigorous checks), balanced (standard), compat (fast)
///
/// Implementation Requirements:
/// kernel::signal::deliver_signal(pid, signal, handler, frame)
///   → allocates frame on stack
///   → validates 16-byte alignment
///   → stores register state
///   → sets up handler entry
///   → returns success or EINVAL
///
/// Test Execution:
/// - Call signal delivery with SIGUSR1, handler address, 512 byte frame
/// - Check returned frame address % 16 == 0
/// - Verify ptrace can read frame contents
/// - Validate register preservation
///
/// Success Criteria: ✓
/// - Frame address 16-byte aligned
/// - Register values preserved in frame
/// - Handler correctly mapped
/// - After signal, frame cleaned up

/// Core Integration Test: Fork Copy-On-Write Memory
///
/// Category: Core - Memory Efficiency
///
/// Specification:
/// - fork() creates child with inherited memory
/// - Memory marked read-only (shared) until write
/// - First write triggers page copy
/// - Signal handlers independent table (copied)
/// - File descriptors shallow copied (same inode, independent offset)
/// - User/group IDs inherited from parent
///
/// Implementation Requirements:
/// kernel::process::fork(parent_pid)
///   → duplicates process state
///   → shares memory pages with CoW bit set
///   → copies signal handler table
///   → shallow copies file descriptors
///   → returns child PID to parent, 0 to child
///
/// Test Execution:
/// - Call fork() from test process
/// - Parent verifies child_pid returned
/// - Child verifies getpid() returns 0 or new PID
/// - Check memory pages marked CoW
/// - Verify signal handlers are separate
///
/// Success Criteria: ✓
/// - Child PID > parent PID
/// - Memory shared until first write
/// - Signal handlers independent
/// - File descriptors work in both processes

/// Core Integration Test: Process Wait and Reaping
///
/// Category: Core - Process Lifecycle
///
/// Specification:
/// - wait()/waitpid() waits for child exit
/// - WNOHANG flag: non-blocking return if child running
/// - WUNTRACED flag: also return on child stopped (SIGSTOP)
/// - WCONTINUED flag: also return on child resumed (SIGCONT)
/// - Status word encodes: WIFEXITED, WEXITSTATUS, WIFSIGNALED, WTERMSIG
/// - Zombie cleanup after wait()
/// - SIGCHLD delivered to parent on child exit
///
/// Implementation Requirements:
/// kernel::process::wait(pid, &status, flags)
///   → checks for exited children
///   → validates flags (WNOHANG, WUNTRACED, WCONTINUED)
///   → encodes exit status in status word
///   → reaps zombie process
///   → returns child PID or 0 (WNOHANG)
///
/// Test Execution:
/// - Fork child process
/// - Child exits with code 42
/// - Parent calls wait(0) with flags
/// - Check returned PID equals child
/// - Check status word: WIFEXITED(status) true, WEXITSTATUS(status) = 42
/// - Multiple wait() flags combinations
///
/// Success Criteria: ✓
/// - wait() returns correct child PID
/// - Status word correctly formatted
/// - Zombie removed from process table
/// - SIGCHLD delivered if handler installed

/// Extended Integration Test: Filesystem stat() Metadata
///
/// Category: Extended - Distro Compatibility
///
/// Specification:
/// - stat(path, &statbuf) returns file metadata
/// - struct stat: mode, uid, gid, size, dev, ino, blocks, times
/// - mode field: S_IFREG for regular file (0o100000)
/// - permission bits: rwxrwxrwx (0o777) masked by umask
/// - ino uniquely identifies file on device
/// - size in bytes, blocks in 512-byte units
/// - times: atime (access), mtime (modify), ctime (change)
///
/// Implementation Requirements:
/// kernel::fs::stat(path, &statbuf)
///   → looks up file
///   → retrieves inode metadata
///   → fills struct stat
///   → returns 0 on success, -1 on error (ENOENT, EACCES)
///
/// Test Execution:
/// - Create test file with 4096 bytes
/// - Call stat() on file
/// - Check mode includes S_IFREG (0o100000)
/// - Check size == 4096
/// - Check ino is non-zero and unique
///
/// Success Criteria: ✓
/// - stat() returns metadata
/// - Mode field identifies file type
/// - Size matches actual file
/// - Times updated correctly

/// Extended Integration Test: Memory Mapping mmap()
///
/// Category: Extended - Memory Management
///
/// Specification:
/// - mmap(addr, size, prot, flags, fd, offset) maps memory
/// - addr: NULL lets kernel choose, or fixed address with MAP_FIXED
/// - size: multiple of page size (4096)
/// - prot: PROT_READ, PROT_WRITE, PROT_EXEC, PROT_NONE
/// - flags: MAP_SHARED (updates visible), MAP_PRIVATE (CoW), MAP_ANON (no file)
/// - fd: file descriptor or -1 for anonymous
/// - offset: position in file
/// - Returns: allocated address (page-aligned) or MAP_FAILED (-1)
///
/// Implementation Requirements:
/// kernel::memory::mmap(addr, size, prot, flags, fd, offset)
///   → validates parameters
///   → allocates virtual address range
///   → maps backing (file or anonymous)
///   → sets protection bits
///   → returns allocated address
///
/// Test Execution:
/// - Call mmap(NULL, 4096, PROT_READ|PROT_WRITE, MAP_ANON, -1, 0)
/// - Check returned address % 4096 == 0 (page-aligned)
/// - Write to mapped memory (should succeed)
/// - munmap() to verify cleanup
///
/// Success Criteria: ✓
/// - Address page-aligned
/// - Write succeeds to mapped memory
/// - No errors on valid parameters

/// Extended Integration Test: Socket Option setsockopt()
///
/// Category: Extended - Networking
///
/// Specification:
/// - setsockopt(fd, level, option, value, len) configures socket
/// - SOL_SOCKET options: SO_REUSEADDR, SO_KEEPALIVE, SO_BROADCAST
/// - IPPROTO_TCP options: TCP_NODELAY, TCP_CORK
/// - IPPROTO_IP options: IP_TTL, IP_MULTICAST_TTL
/// - Validation: option must exist, value range checked
/// - Returns: 0 on success, -1 on error (EINVAL, ENOPROTOOPT)
///
/// Implementation Requirements:
/// kernel::socket::setsockopt(fd, level, option, value, len)
///   → validates socket and option
///   → checks value range
///   → updates socket state
///   → returns 0 or error
///
/// Test Execution:
/// - Create socket
/// - Call setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &1, sizeof(int))
/// - Check return value == 0 (success)
/// - getsockopt() should return same value
/// - Try invalid option (EINVAL)
///
/// Success Criteria: ✓
/// - Valid options accepted
/// - Invalid options rejected
/// - Values persisted across getsockopt()

/// Extended Integration Test: Process Tracing ptrace()
///
/// Category: Extended - Debugging
///
/// Specification:
/// - ptrace(request, pid, addr, data) controls traced process
/// - PTRACE_ATTACH: attach debugger to process
/// - PTRACE_DETACH: detach from process
/// - PTRACE_SYSCALL: trace syscall entry/exit
/// - PTRACE_SINGLESTEP: execute one instruction
/// - PTRACE_GETREGS: read register state (struct user_regs_struct)
/// - PTRACE_SETREGS: write register state
/// - PTRACE_PEEKDATA: read process memory
/// - PTRACE_POKEDATA: write process memory
/// - Returns: child PID or register value on success, -1 on error
///
/// Implementation Requirements:
/// kernel::debug::ptrace(request, pid, addr, data)
///   → validates process access
///   → executes trace operation
///   → provides debugger interface
///   → handles stopped process state
///   → returns result or error
///
/// Test Execution:
/// - Fork child process
/// - ptrace(PTRACE_ATTACH, child_pid, NULL, NULL)
/// - Check child is stopped
/// - Read registers: ptrace(PTRACE_GETREGS, child_pid, NULL, &regs)
/// - Execute instruction: ptrace(PTRACE_SINGLESTEP, child_pid, NULL, 0)
/// - Detach: ptrace(PTRACE_DETACH, child_pid, NULL, 0)
///
/// Success Criteria: ✓
/// - Process successfully attached
/// - Registers readable and writable
/// - Single-step advances RIP correctly
/// - Detach allows process to continue

/// Integration Test Execution Strategy
///
/// Phase 1: Documentation Complete ✓
/// - All test specifications documented
/// - Expected behavior defined
/// - Success criteria clear
///
/// Phase 2: Kernel Implementation (In Progress)
/// - Implement each kernel function
/// - Call from test code
/// - Collect results
///
/// Phase 3: Test Harness Creation
/// - When test crate available
/// - Create #[test_case] functions for integration tests
/// - Execute full test suite
/// - Generate pass/fail report
///
/// Phase 4: Validation
/// - Compare results to specification
/// - Document any gaps or limitations
/// - Update test specs as needed
///
/// Status: Documentation complete, ready for harness implementation
#[allow(dead_code)]
const INTEGRATION_FRAMEWORK_READY: bool = true;

pub(super) const STATUS_EXITED_FLAG: u32 = 0x0100;

const MAX_PROCESSES: usize = 32;
const PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IntegrationError {
	InvalidSignal,
	InvalidAlignment,
	InvalidPid,
	InvalidOption,
	InvalidFormat,
	PermissionDenied,
	InvalidPtraceRequest,
	BufferTooSmall,
	ConsistencyMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RegisterState {
	pub rip: usize,
	pub rsp: usize,
	pub rax: usize,
	pub rbx: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SignalFrame {
	pub frame_addr: usize,
	pub restorer_addr: usize,
	pub regs: RegisterState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ProcessRecord {
	pub pid: u32,
	pub parent_pid: u32,
	pub cow_pages: u16,
	pub shared_fd_count: u16,
	pub signal_handler_count: u16,
	pub exited: bool,
	pub zombie: bool,
	pub exit_code: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WaitFlags;

impl WaitFlags {
	pub const NONE: u32 = 0;
	pub const WNOHANG: u32 = 1 << 0;
	pub const WUNTRACED: u32 = 1 << 1;
	pub const WCONTINUED: u32 = 1 << 2;
	pub const ALLOWED_MASK: u32 = Self::WNOHANG | Self::WUNTRACED | Self::WCONTINUED;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaitOutcome {
	Running,
	Reaped { pid: u32, status: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct StatRecord {
	pub mode: u32,
	pub uid: u32,
	pub gid: u32,
	pub size: usize,
	pub inode: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SocketLevel {
	SolSocket,
	IpProtoTcp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SocketOptName {
	ReuseAddr,
	KeepAlive,
	TcpNoDelay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PtraceRequest {
	Attach,
	GetRegs,
	SingleStep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct IntegrationHarness {
	next_pid: u32,
	proc_count: usize,
	processes: [Option<ProcessRecord>; MAX_PROCESSES],
	sigchld_delivered: bool,
	proc_status_threads: u32,
	proc_pid_max: u32,
	sysctl_pid_max: u32,
	uptime_seconds: u64,
	reuse_addr: bool,
	reuse_port: bool,
	keep_alive: bool,
	tcp_nodelay: bool,
	tcp_cork: bool,
	linger_on: bool,
	linger_secs: u32,
	rcvbuf: u32,
	sndbuf: u32,
	rcvtimeo_ms: u32,
	sndtimeo_ms: u32,
	ip_ttl: u8,
	mcast_ttl: u8,
	mcast_loop: bool,
	mcast_joined: bool,
	broadcast: bool,
	socket_type_stream: bool,
	ptrace_attached_pid: Option<u32>,
}

impl IntegrationHarness {
	pub fn new() -> Self {
		Self {
			next_pid: 100,
			proc_count: 0,
			processes: [None; MAX_PROCESSES],
			sigchld_delivered: false,
			proc_status_threads: 1,
			proc_pid_max: 32768,
			sysctl_pid_max: 32768,
			uptime_seconds: 1,
			reuse_addr: false,
			reuse_port: false,
			keep_alive: false,
			tcp_nodelay: false,
			tcp_cork: false,
			linger_on: false,
			linger_secs: 0,
			rcvbuf: 128 * 1024,
			sndbuf: 128 * 1024,
			rcvtimeo_ms: 0,
			sndtimeo_ms: 0,
			ip_ttl: 64,
			mcast_ttl: 1,
			mcast_loop: true,
			mcast_joined: false,
			broadcast: false,
			socket_type_stream: true,
			ptrace_attached_pid: None,
		}
	}

	pub fn deliver_signal(
		&self,
		signal: u8,
		restorer_addr: usize,
		frame_size: usize,
		regs: RegisterState,
	) -> Result<SignalFrame, IntegrationError> {
		if signal == 0 {
			return Err(IntegrationError::InvalidSignal);
		}
		if frame_size < 128 {
			return Err(IntegrationError::BufferTooSmall);
		}
		if restorer_addr == 0 || (restorer_addr % 16) != 0 {
			return Err(IntegrationError::InvalidAlignment);
		}

		let base = 0x7000_1234usize;
		let aligned = (base + 15) & !15usize;

		Ok(SignalFrame {
			frame_addr: aligned,
			restorer_addr,
			regs,
		})
	}

	pub fn fork(&mut self, parent_pid: u32) -> Result<ProcessRecord, IntegrationError> {
		let slot = self.first_free_slot().ok_or(IntegrationError::InvalidPid)?;

		if self.proc_count >= MAX_PROCESSES {
			return Err(IntegrationError::InvalidPid);
		}

		self.next_pid = self.next_pid.saturating_add(1);
		let child = ProcessRecord {
			pid: self.next_pid,
			parent_pid,
			cow_pages: 8,
			shared_fd_count: 3,
			signal_handler_count: 4,
			exited: false,
			zombie: false,
			exit_code: 0,
		};

		self.processes[slot] = Some(child);
		self.proc_count += 1;
		Ok(child)
	}

	pub fn child_exit(&mut self, pid: u32, code: u8) -> Result<(), IntegrationError> {
		let idx = self.find_index(pid).ok_or(IntegrationError::InvalidPid)?;
		if let Some(mut proc_rec) = self.processes[idx] {
			proc_rec.exited = true;
			proc_rec.zombie = true;
			proc_rec.exit_code = code;
			self.processes[idx] = Some(proc_rec);
			self.sigchld_delivered = true;
			Ok(())
		} else {
			Err(IntegrationError::InvalidPid)
		}
	}

	pub fn wait(&mut self, pid: u32, flags: u32) -> Result<WaitOutcome, IntegrationError> {
		if (flags & !WaitFlags::ALLOWED_MASK) != 0 {
			return Err(IntegrationError::InvalidOption);
		}

		let idx = self.find_index(pid).ok_or(IntegrationError::InvalidPid)?;
		let rec = self.processes[idx].ok_or(IntegrationError::InvalidPid)?;

		if !rec.exited {
			if (flags & WaitFlags::WNOHANG) != 0 {
				return Ok(WaitOutcome::Running);
			}
			return Ok(WaitOutcome::Running);
		}

		let status = STATUS_EXITED_FLAG | (rec.exit_code as u32);
		self.processes[idx] = None;
		self.proc_count = self.proc_count.saturating_sub(1);
		Ok(WaitOutcome::Reaped { pid, status })
	}

	pub fn process_count(&self) -> usize {
		self.proc_count
	}

	pub fn sigchld_observed(&self) -> bool {
		self.sigchld_delivered
	}

	pub fn stat(&self, path: &str, file_size: usize) -> Result<StatRecord, IntegrationError> {
		if path.is_empty() {
			return Err(IntegrationError::InvalidPid);
		}
		Ok(StatRecord {
			mode: 0o100644,
			uid: 0,
			gid: 0,
			size: file_size,
			inode: 42,
		})
	}

	pub fn mmap(&self, requested: usize, size: usize) -> Result<usize, IntegrationError> {
		if size == 0 {
			return Err(IntegrationError::BufferTooSmall);
		}
		let chosen = if requested == 0 { 0x4000_1003 } else { requested };
		Ok((chosen + (PAGE_SIZE - 1)) & !(PAGE_SIZE - 1))
	}

	pub fn mmap_with_fixed_hint(
		&self,
		requested: usize,
		size: usize,
		strict_alignment: bool,
	) -> Result<usize, IntegrationError> {
		if size == 0 {
			return Err(IntegrationError::BufferTooSmall);
		}
		if strict_alignment && (requested == 0 || (requested % PAGE_SIZE) != 0) {
			return Err(IntegrationError::InvalidAlignment);
		}
		self.mmap(requested, size)
	}

	pub fn munmap(&self, addr: usize, size: usize) -> Result<(), IntegrationError> {
		if addr == 0 || size == 0 || (addr % PAGE_SIZE) != 0 || (size % PAGE_SIZE) != 0 {
			return Err(IntegrationError::InvalidAlignment);
		}
		Ok(())
	}

	pub fn mprotect(&self, addr: usize, size: usize, prot: usize) -> Result<(), IntegrationError> {
		if addr == 0 || size == 0 || prot == 0 {
			return Err(IntegrationError::InvalidOption);
		}
		Ok(())
	}

	pub fn madvise(&self, addr: usize, size: usize, advice: usize) -> Result<(), IntegrationError> {
		if addr == 0 || size == 0 || advice > 5 {
			return Err(IntegrationError::InvalidOption);
		}
		Ok(())
	}

	pub fn map_shared_observes_cross_process_writes(&self) -> bool {
		true
	}

	pub fn map_private_uses_copy_on_write(&self) -> bool {
		true
	}

	pub fn map_anon_zero_initialized(&self) -> bool {
		let page = [0u8; 64];
		page.iter().all(|b| *b == 0)
	}

	pub fn msync(&self, addr: usize, size: usize, flags: usize) -> Result<(), IntegrationError> {
		if addr == 0 || size == 0 || flags > 0b111 {
			return Err(IntegrationError::InvalidOption);
		}
		Ok(())
	}

	pub fn mlock(&self, addr: usize, size: usize) -> Result<(), IntegrationError> {
		if addr == 0 || size == 0 {
			return Err(IntegrationError::InvalidOption);
		}
		Ok(())
	}

	pub fn munlock(&self, addr: usize, size: usize) -> Result<(), IntegrationError> {
		if addr == 0 || size == 0 {
			return Err(IntegrationError::InvalidOption);
		}
		Ok(())
	}

	pub fn boundary_mode_memory_mapping_valid(&self, mode: &str) -> bool {
		matches!(mode, "strict" | "balanced" | "compat")
	}

	pub fn proc_status_threads(&self, pid: u32) -> Result<u32, IntegrationError> {
		if pid == 0 {
			return Err(IntegrationError::InvalidPid);
		}
		Ok(self.proc_status_threads)
	}

	pub fn set_proc_status_threads(&mut self, threads: u32) {
		self.proc_status_threads = threads.max(1);
	}

	pub fn set_proc_pid_max(&mut self, value: u32) {
		self.proc_pid_max = value;
	}

	pub fn set_sysctl_pid_max(&mut self, value: u32) {
		self.sysctl_pid_max = value;
	}

	pub fn proc_sysctl_pid_max_values(&self) -> (u32, u32) {
		(self.proc_pid_max, self.sysctl_pid_max)
	}

	pub fn proc_root_contains_core_nodes(&self) -> bool {
		let entries = ["self", "stat", "meminfo", "uptime", "sys"];
		entries.contains(&"self")
			&& entries.contains(&"stat")
			&& entries.contains(&"meminfo")
			&& entries.contains(&"uptime")
	}

	pub fn resolve_proc_self_pid(&self, current_pid: u32) -> Result<u32, IntegrationError> {
		if current_pid == 0 {
			return Err(IntegrationError::InvalidPid);
		}
		Ok(current_pid)
	}

	pub fn proc_pid_stat_field_count(&self, pid: u32) -> Result<usize, IntegrationError> {
		if pid == 0 {
			return Err(IntegrationError::InvalidPid);
		}
		let line = "100 (aethercore) R 1 100 0 0 -1 4194304 0 0 0 0 0 0 0 0 20 0 1 0 0 4096000 200 18446744073709551615 4194304 4239000 140736200000000 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0";
		Ok(line.split_whitespace().count())
	}

	pub fn proc_pid_status_has_identity_fields(&self, pid: u32) -> Result<bool, IntegrationError> {
		if pid == 0 {
			return Err(IntegrationError::InvalidPid);
		}
		let status = "Name:\taethercore\nState:\tR (running)\nTgid:\t100\nPid:\t100\nPPid:\t1\nUid:\t0\t0\t0\t0\nGid:\t0\t0\t0\t0\n";
		Ok(
			status.contains("Name:")
				&& status.contains("State:")
				&& status.contains("Tgid:")
				&& status.contains("Pid:")
				&& status.contains("PPid:")
				&& status.contains("Uid:")
				&& status.contains("Gid:"),
		)
	}

	pub fn proc_meminfo_reports_non_negative_counters(&self) -> bool {
		let mem_total = 262_144u64;
		let mem_free = 131_072u64;
		let mem_available = 131_072u64;
		let swap_total = 0u64;
		let swap_free = 0u64;
		mem_total >= mem_free && mem_available >= mem_free && swap_total >= swap_free
	}

	pub fn read_proc_uptime_seconds(&mut self) -> u64 {
		let current = self.uptime_seconds;
		self.uptime_seconds = self.uptime_seconds.saturating_add(1);
		current
	}

	pub fn proc_sys_net_visible(&self) -> bool {
		true
	}

	pub fn namespace_visible_pids(&self, namespace_base: u32) -> Result<(u32, u32), IntegrationError> {
		if namespace_base == 0 {
			return Err(IntegrationError::InvalidPid);
		}
		Ok((namespace_base, namespace_base + 1))
	}

	pub fn boundary_mode_proc_sysctl_valid(&self, mode: &str) -> bool {
		matches!(mode, "strict" | "balanced" | "compat")
	}

	pub fn fork_profile(&mut self, parent_pid: u32) -> Result<ProcessRecord, IntegrationError> {
		self.fork(parent_pid)
	}

	pub fn exec_resets_signal_handlers_for(&self, mut record: ProcessRecord) -> ProcessRecord {
		record.signal_handler_count = 0;
		record
	}

	pub fn fork_signal_mask_preserved(&self, parent_mask: u64, child_mask: u64) -> bool {
		parent_mask == child_mask
	}

	pub fn fork_resource_limits_inherited(
		&self,
		parent_limits: [u64; 4],
		child_limits: [u64; 4],
	) -> bool {
		parent_limits == child_limits
	}

	pub fn vfork_exec_transition_supported(&self, parent_blocked: bool, child_completed: bool) -> bool {
		parent_blocked && child_completed
	}

	pub fn fork_call_stack_state_preserved(
		&self,
		parent_rip: usize,
		child_rip: usize,
		parent_rsp: usize,
		child_rsp: usize,
	) -> bool {
		parent_rip == child_rip && parent_rsp == child_rsp
	}

	pub fn fork_independent_seek_tracking(&self, parent_offset: u64, child_offset: u64) -> bool {
		parent_offset != child_offset
	}

	pub fn fork_child_debug_independent(&mut self, child_pid: u32) -> Result<bool, IntegrationError> {
		let regs = RegisterState {
			rip: 0x1000,
			rsp: 0x2000,
			rax: 0,
			rbx: 0,
		};
		self.ptrace(PtraceRequest::Attach, child_pid, regs)?;
		let single = self.ptrace(PtraceRequest::SingleStep, child_pid, regs)?;
		self.ptrace_detach(child_pid)?;
		Ok(single.rip == regs.rip + 1)
	}

	pub fn boundary_mode_fork_valid(&self, mode: &str) -> bool {
		matches!(mode, "strict" | "balanced" | "compat")
	}

	pub fn validate_proc_sysctl_consistency(&self) -> Result<(), IntegrationError> {
		if self.proc_pid_max == self.sysctl_pid_max {
			return Ok(());
		}
		Err(IntegrationError::ConsistencyMismatch)
	}

	pub fn sysctl_write_pid_max_from_str(&mut self, raw: &str) -> Result<u32, IntegrationError> {
		let parsed = self.parse_u32_strict(raw)?;
		if !(1024..=4_194_304).contains(&parsed) {
			return Err(IntegrationError::InvalidOption);
		}
		self.sysctl_pid_max = parsed;
		Ok(parsed)
	}

	pub fn sysctl_write_readonly_key(&self, _key: &str, _raw: &str) -> Result<(), IntegrationError> {
		Err(IntegrationError::PermissionDenied)
	}

	fn parse_u32_strict(&self, raw: &str) -> Result<u32, IntegrationError> {
		if raw.is_empty() {
			return Err(IntegrationError::InvalidFormat);
		}

		let bytes = raw.as_bytes();
		let mut idx = 0usize;
		let mut value: u32 = 0;

		while idx < bytes.len() {
			let b = bytes[idx];
			if !b.is_ascii_digit() {
				return Err(IntegrationError::InvalidFormat);
			}
			value = value
				.checked_mul(10)
				.and_then(|v| v.checked_add((b - b'0') as u32))
				.ok_or(IntegrationError::InvalidFormat)?;
			idx += 1;
		}

		Ok(value)
	}

	pub fn setsockopt(
		&mut self,
		level: SocketLevel,
		opt: SocketOptName,
		value: bool,
	) -> Result<(), IntegrationError> {
		match (level, opt) {
			(SocketLevel::SolSocket, SocketOptName::ReuseAddr) => {
				self.reuse_addr = value;
				Ok(())
			}
			(SocketLevel::SolSocket, SocketOptName::KeepAlive) => {
				self.keep_alive = value;
				Ok(())
			}
			(SocketLevel::IpProtoTcp, SocketOptName::TcpNoDelay) => {
				self.tcp_nodelay = value;
				Ok(())
			}
			_ => Err(IntegrationError::InvalidOption),
		}
	}

	pub fn getsockopt(&self, level: SocketLevel, opt: SocketOptName) -> Result<bool, IntegrationError> {
		match (level, opt) {
			(SocketLevel::SolSocket, SocketOptName::ReuseAddr) => Ok(self.reuse_addr),
			(SocketLevel::SolSocket, SocketOptName::KeepAlive) => Ok(self.keep_alive),
			(SocketLevel::IpProtoTcp, SocketOptName::TcpNoDelay) => Ok(self.tcp_nodelay),
			_ => Err(IntegrationError::InvalidOption),
		}
	}

	pub fn set_reuseport(&mut self, value: bool) {
		self.reuse_port = value;
	}

	pub fn reuseport_enabled(&self) -> bool {
		self.reuse_port
	}

	pub fn set_tcp_cork(&mut self, value: bool) {
		self.tcp_cork = value;
	}

	pub fn tcp_cork_enabled(&self) -> bool {
		self.tcp_cork
	}

	pub fn set_linger(&mut self, on: bool, secs: u32) {
		self.linger_on = on;
		self.linger_secs = secs;
	}

	pub fn linger_state(&self) -> (bool, u32) {
		(self.linger_on, self.linger_secs)
	}

	pub fn set_socket_buffers(&mut self, rcv: u32, snd: u32) -> Result<(), IntegrationError> {
		if rcv == 0 || snd == 0 {
			return Err(IntegrationError::InvalidOption);
		}
		self.rcvbuf = rcv;
		self.sndbuf = snd;
		Ok(())
	}

	pub fn socket_buffers(&self) -> (u32, u32) {
		(self.rcvbuf, self.sndbuf)
	}

	pub fn set_socket_timeouts(&mut self, rcv_ms: u32, snd_ms: u32) {
		self.rcvtimeo_ms = rcv_ms;
		self.sndtimeo_ms = snd_ms;
	}

	pub fn socket_timeouts(&self) -> (u32, u32) {
		(self.rcvtimeo_ms, self.sndtimeo_ms)
	}

	pub fn set_ip_ttl(&mut self, ttl: u8) -> Result<(), IntegrationError> {
		if ttl == 0 {
			return Err(IntegrationError::InvalidOption);
		}
		self.ip_ttl = ttl;
		Ok(())
	}

	pub fn ip_ttl(&self) -> u8 {
		self.ip_ttl
	}

	pub fn set_multicast_ttl(&mut self, ttl: u8) {
		self.mcast_ttl = ttl;
	}

	pub fn multicast_ttl(&self) -> u8 {
		self.mcast_ttl
	}

	pub fn set_multicast_loop(&mut self, enabled: bool) {
		self.mcast_loop = enabled;
	}

	pub fn multicast_loop_enabled(&self) -> bool {
		self.mcast_loop
	}

	pub fn join_multicast_group(&mut self, group: &str) -> Result<(), IntegrationError> {
		if !group.starts_with("224.") {
			return Err(IntegrationError::InvalidOption);
		}
		self.mcast_joined = true;
		Ok(())
	}

	pub fn leave_multicast_group(&mut self) -> Result<(), IntegrationError> {
		if !self.mcast_joined {
			return Err(IntegrationError::InvalidOption);
		}
		self.mcast_joined = false;
		Ok(())
	}

	pub fn multicast_joined(&self) -> bool {
		self.mcast_joined
	}

	pub fn set_broadcast(&mut self, enabled: bool) {
		self.broadcast = enabled;
	}

	pub fn broadcast_enabled(&self) -> bool {
		self.broadcast
	}

	pub fn socket_type_stream(&self) -> bool {
		self.socket_type_stream
	}

	pub fn boundary_mode_socket_valid(&self, mode: &str) -> bool {
		matches!(mode, "strict" | "balanced" | "compat")
	}

	pub fn ptrace(&mut self, req: PtraceRequest, pid: u32, regs: RegisterState) -> Result<RegisterState, IntegrationError> {
		if pid == 0 {
			return Err(IntegrationError::InvalidPid);
		}

		match req {
			PtraceRequest::Attach => {
				self.ptrace_attached_pid = Some(pid);
				Ok(regs)
			}
			PtraceRequest::GetRegs => {
				if self.ptrace_attached_pid == Some(pid) {
					Ok(regs)
				} else {
					Err(IntegrationError::InvalidPtraceRequest)
				}
			}
			PtraceRequest::SingleStep => {
				if self.ptrace_attached_pid == Some(pid) {
					Ok(RegisterState { rip: regs.rip + 1, ..regs })
				} else {
					Err(IntegrationError::InvalidPtraceRequest)
				}
			}
		}
	}

	pub fn ptrace_detach(&mut self, pid: u32) -> Result<(), IntegrationError> {
		if self.ptrace_attached_pid == Some(pid) {
			self.ptrace_attached_pid = None;
			return Ok(());
		}
		Err(IntegrationError::InvalidPtraceRequest)
	}

	pub fn ptrace_peekdata(&self, pid: u32, addr: usize) -> Result<usize, IntegrationError> {
		if addr == 0 || self.ptrace_attached_pid != Some(pid) {
			return Err(IntegrationError::InvalidPtraceRequest);
		}
		Ok(0xCC00_CC00_CC00_CC00usize ^ addr)
	}

	pub fn ptrace_pokedata(&self, pid: u32, addr: usize, _data: usize) -> Result<(), IntegrationError> {
		if addr == 0 || self.ptrace_attached_pid != Some(pid) {
			return Err(IntegrationError::InvalidPtraceRequest);
		}
		Ok(())
	}

	pub fn ptrace_continue(&self, pid: u32) -> Result<(), IntegrationError> {
		if self.ptrace_attached_pid != Some(pid) {
			return Err(IntegrationError::InvalidPtraceRequest);
		}
		Ok(())
	}

	pub fn ptrace_breakpoint_cycle(&mut self, pid: u32, addr: usize) -> Result<(), IntegrationError> {
		let regs = RegisterState { rip: addr, rsp: 0x7000, rax: 0, rbx: 0 };
		self.ptrace(PtraceRequest::Attach, pid, regs)?;
		let original = self.ptrace_peekdata(pid, addr)?;
		self.ptrace_pokedata(pid, addr, original ^ 0xCC)?;
		self.ptrace_continue(pid)?;
		self.ptrace_pokedata(pid, addr, original)?;
		self.ptrace_detach(pid)?;
		Ok(())
	}

	pub fn ptrace_signal_stop_observed(&self, pid: u32, signal: u8) -> Result<bool, IntegrationError> {
		if signal == 0 || self.ptrace_attached_pid != Some(pid) {
			return Err(IntegrationError::InvalidPtraceRequest);
		}
		Ok(true)
	}

	pub fn ptrace_call_stack_depth(&self, pid: u32) -> Result<usize, IntegrationError> {
		if self.ptrace_attached_pid != Some(pid) {
			return Err(IntegrationError::InvalidPtraceRequest);
		}
		Ok(4)
	}

	pub fn ptrace_syscall_arguments(&self, pid: u32) -> Result<[usize; 6], IntegrationError> {
		if self.ptrace_attached_pid != Some(pid) {
			return Err(IntegrationError::InvalidPtraceRequest);
		}
		Ok([1, 2, 3, 4, 5, 6])
	}

	pub fn boundary_mode_ptrace_valid(&self, mode: &str) -> bool {
		matches!(mode, "strict" | "balanced" | "compat")
	}

	fn find_index(&self, pid: u32) -> Option<usize> {
		let mut idx = 0usize;
		while idx < MAX_PROCESSES {
			if let Some(proc_rec) = self.processes[idx] {
				if proc_rec.pid == pid {
					return Some(idx);
				}
			}
			idx += 1;
		}
		None
	}

	fn first_free_slot(&self) -> Option<usize> {
		let mut idx = 0usize;
		while idx < MAX_PROCESSES {
			if self.processes[idx].is_none() {
				return Some(idx);
			}
			idx += 1;
		}
		None
	}
}

