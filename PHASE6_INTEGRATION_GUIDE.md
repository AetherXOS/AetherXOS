# Phase 6 Integration Guide: Wiring Extensions into kernel_runtime

## Overview
Phase 6 focuses on integrating all Phase 4-5 implementations into the actual boot sequence
and runtime managers. This document provides a step-by-step integration checklist.

## Phase 6 Tasks

### Task 1: kernel_runtime.rs Integration (Priority: CRITICAL)

#### 1.1 Import Boot Infrastructure
```rust
use crate::kernel::boot_manager::{GLOBAL_BOOT_MANAGER, ConcreteBootManager};
use crate::kernel::boot_subsystems::*;
use crate::kernel::runtime_manager::{GLOBAL_RUNTIME_MANAGER, RuntimeState};
use crate::kernel::device_manager::GLOBAL_DEVICE_MANAGER;
```

#### 1.2 Boot Stage Orchestration
Current structure expects linear boot flow. Phase 6 should:
1. Replace manual subsystem initialization with GLOBAL_BOOT_MANAGER calls
2. Register all 7 boot subsystems at each appropriate stage
3. Track timing via record_stage_timing()
4. Validate dependencies before entering each stage

#### 1.3 Stage Transition Points
- BootStage::BootloaderHandoff: Set RuntimeState::Initializing
- BootStage::EarlyMemory: Register ALLOCATOR_SUBSYSTEM
- BootStage::PlatformEarly: Call Platform::init(), register SCHEDULER/INTERRUPT_SUBSYSTEM
- BootStage::PlatformDevices: Register device enumeration
- BootStage::CoreSubsystems: Register VFS/IPC/SECURITY/PROCESS_SUBSYSTEMS
- BootStage::UserspaceReady: Set RuntimeState::Ready
- BootStage::ShutdownInit: Set RuntimeState::Shutdown

#### 1.4 Error Handling
- Capture boot diagnostics after each stage
- Log stage_timings and stage_errors
- Prevent forward progress if critical subsystems fail

### Task 2: Platform Initialization (Priority: HIGH)

#### 2.1 Early Platform Detection
At PlatformEarly stage:
```rust
if cfg!(target_arch = "x86_64") {
    crate::hal::platforms::X86_64_PLATFORM.init()?;
} else if cfg!(target_arch = "aarch64") {
    crate::hal::platforms::AARCH64_PLATFORM.init()?;
}
```

#### 2.2 CPU Feature Recording
- Call platform.capabilities() to get:
  - cpu_count
  - has_smp
  - has_virtualization
  - has_fpu / has_sse / has_avx (x86_64)
- Record in RuntimeConfig for later verification

#### 2.3 Memory Layout Validation
- Get platform.memory_layout()
- Verify kernel base address matches compilation settings
- Check virtual bias for address translation

### Task 3: Device Manager Integration (Priority: HIGH)

#### 3.1 Device Enumeration
At PlatformDevices stage:
1. Enumerate fixed devices (serial, timer, CPU cores)
2. For x86_64: Parse ACPI for dynamic devices
3. For aarch64: Parse device tree blob (DTB)
4. Register via GLOBAL_DEVICE_MANAGER.register()

#### 3.2 Device Initialization
```rust
for device_id in GLOBAL_DEVICE_MANAGER.all_device_ids() {
    if let Some((device_type, state)) = GLOBAL_DEVICE_MANAGER.get_device(device_id) {
        if matches!(state, DeviceState::Uninitialized) {
            device_init_by_type(device_type, device_id)?;
            GLOBAL_DEVICE_MANAGER.set_device_state(device_id, DeviceState::Ready)?;
        }
    }
}
```

#### 3.3 Device Type Handlers
- **Timer**: Initialize tick frequency, set up interrupt handler
- **Serial**: Enable UART, configure baud rate, redirect log::* to serial
- **CPU**: Detect topology, enable SMP if available
- **Interrupt Controller**: Program APIC/GIC, enable interrupts

### Task 4: Scheduler Integration (Priority: HIGH)

#### 4.1 Priority Scheduler Hookup
At CoreSubsystems stage:
```rust
// For each task at creation (fork/exec):
PRIORITY_SCHEDULER.set_task_priority(task_id, PriorityLevel::Interactive);

// For real-time tasks:
REALTIME_SCHEDULER.set_realtime_deadline(task_id, deadline_ns)?;
```

#### 4.2 Real-Time Admission
```rust
// In sys_fork/sys_exec:
if is_realtime_request {
    match REALTIME_SCHEDULER.can_admit_realtime(task_id, period, runtime) {
        Ok(()) => { /* proceed */ }
        Err("Admission denied") => { /* reject fork */ }
    }
}
```

#### 4.3 Group Scheduling
```rust
// For per-cgroup scheduling:
GROUP_SCHEDULER.create_group(period_ns, quota_ns)?;
GROUP_SCHEDULER.add_task_to_group(group_id, task_id)?;
GROUP_SCHEDULER.set_cpu_affinity(task_id, affinity_mask)?;
```

### Task 5: Memory Subsystem Integration (Priority: HIGH)

#### 5.1 Pressure Handler Hookup
At page allocator level:
```rust
// In page_reclaim() / kswapd:
PRESSURE_HANDLER.update_stats(total_pages, free_pages, cached, dirty);

// Register reclaim callbacks:
PRESSURE_HANDLER.register_pressure_callback(|pressure| {
    match pressure {
        MemoryPressure::High => shrink_caches(100),
        MemoryPressure::Critical => emergency_shrink(1000),
        _ => {}
    }
});
```

#### 5.2 NUMA Allocator Integration
```rust
// At process_create:
match NUMA_ALLOCATOR.allocate_on_node(local_node, pages) {
    Ok(_) => { /* use allocation */ }
    Err(_) => { /* fall back to global */ }
}
```

#### 5.3 Quota Enforcement
```rust
// At page_alloc:
if !MEMORY_ACCOUNTANT.can_allocate(pid, size) {
    return Err("Quota exceeded");
}
MEMORY_ACCOUNTANT.record_allocation(pid, size)?;
```

#### 5.4 QoS Manager
```rust
// At fork/exec:
MEMORY_QOS_MANAGER.set_process_qos(pid, determine_qos_tier(pid))?;

// At allocate:
MEMORY_QOS_MANAGER.allocate_with_qos(pid, size, qos_tier)?;
```

### Task 6: VFS Integration (Priority: MEDIUM)

#### 6.1 Permission Manager Hookup
```rust
// At inode_permission check:
if !PERMISSION_MANAGER.check_permission(inode, uid, gid, action) {
    return Err("Permission denied");
}
```

#### 6.2 Mount Manager Integration
```rust
// At sys_mount:
MOUNT_MANAGER.mount(mount_path, fstype, source, options)?;

// At sys_umount:
MOUNT_MANAGER.unmount(mount_path)?;
```

#### 6.3 Quota Enforcement
```rust
// At block_alloc:
if !QUOTA_MANAGER.can_allocate(uid, blocks) {
    return Err("Block quota exceeded");
}
```

### Task 7: Device Enumeration Modules (Priority: MEDIUM)

#### 7.1 ACPI Parser (x86_64)
- Create: kernel/src/hal/acpi_parser.rs
- Parse ACPI tables from BIOS-provided pointer
- Enumerate PCI devices, platform devices
- Register with GLOBAL_DEVICE_MANAGER

#### 7.2 Device Tree Parser (aarch64)
- Create: kernel/src/hal/dtb_parser.rs
- Parse device tree blob from bootloader
- Extract device nodes and properties
- Register with GLOBAL_DEVICE_MANAGER

### Task 8: Boot Subsystem Real Initialization (Priority: MEDIUM)

#### 8.1 Update boot_subsystems.rs init() methods
```rust
// Instead of stub implementations:
pub fn init(&self) -> Result<(), &'static str> {
    // Call actual subsystem initialization code
    allocator::init()?;  // or similar
    self.set_ready(true)?;
    Ok(())
}
```

#### 8.2 Hook each subsystem's real init
- AllocatorBootSubsystem.init() → allocator::init()
- SchedulerBootSubsystem.init() → scheduler::init()
- VfsBootSubsystem.init() → vfs::init()
- etc.

### Task 9: Integration Tests (Priority: LOW)

#### 9.1 Boot Sequence Tests
```rust
#[test]
fn test_complete_boot_sequence() {
    GLOBAL_BOOT_MANAGER.enter_stage(BootStage::BootloaderHandoff)?;
    GLOBAL_BOOT_MANAGER.enter_stage(BootStage::EarlyMemory)?;
    GLOBAL_BOOT_MANAGER.enter_stage(BootStage::PlatformEarly)?;
    // ... all 8 stages
    assert_eq!(GLOBAL_RUNTIME_MANAGER.state(), RuntimeState::Ready);
}
```

#### 9.2 Dependency Verification
```rust
#[test]
fn test_boot_dependencies() {
    // Verify each stage's dependencies are satisfied before entry
    let diags = GLOBAL_BOOT_MANAGER.diagnostics();
    assert!(diags.stage_errors.is_empty());
}
```

#### 9.3 Subsystem Availability
```rust
#[test]
fn test_all_subsystems_ready() {
    // After UserspaceReady stage:
    assert!(ALLOCATOR_SUBSYSTEM.is_ready());
    assert!(SCHEDULER_SUBSYSTEM.is_ready());
    // ... etc
}
```

## Integration Checklist

### Phase 6A: Core Boot Sequence
- [ ] Import GLOBAL_BOOT_MANAGER in kernel_runtime.rs
- [ ] Register all 7 boot subsystems at EarlyMemory stage
- [ ] Implement stage transition points (8 total)
- [ ] Add error handling and diagnostics
- [ ] Add timing collection
- [ ] Test boot flow with mock devices

### Phase 6B: Platform Integration
- [ ] Call Platform::init() at PlatformEarly
- [ ] Query platform capabilities
- [ ] Validate memory layout
- [ ] Record CPU count and features
- [ ] Test on both x86_64 and aarch64

### Phase 6C: Device Management
- [ ] Integrate GLOBAL_DEVICE_MANAGER
- [ ] Enumerate fixed devices
- [ ] Add ACPI parser for x86_64
- [ ] Add DTB parser for aarch64
- [ ] Register enumerated devices
- [ ] Test device discovery

### Phase 6D: Scheduler Integration
- [ ] Wire PriorityScheduler to task creation
- [ ] Implement real-time admission control
- [ ] Add group scheduling hooks
- [ ] Test priority assignment
- [ ] Verify admission enforcement

### Phase 6E: Memory Integration
- [ ] Wire MemoryPressureHandler to page allocator
- [ ] Hook NUMA allocator
- [ ] Integrate quota enforcement
- [ ] Enable QoS tier selection
- [ ] Test pressure callbacks

### Phase 6F: VFS Integration
- [ ] Connect permission manager
- [ ] Integrate mount manager
- [ ] Wire quota enforcement
- [ ] Test permission checks
- [ ] Verify mount operations

### Phase 6G: Subsystem Initialization
- [ ] Update boot_subsystems.rs init() methods
- [ ] Call real subsystem setup code
- [ ] Verify subsystem readiness
- [ ] Collect initialization timings
- [ ] Test full subsystem startup

### Phase 6H: Testing & Validation
- [ ] Integration tests for boot sequence
- [ ] Dependency verification tests
- [ ] End-to-end boot tests
- [ ] Performance profiling
- [ ] Hardware validation (QEMU)

## Success Criteria for Phase 6

✅ All 130+ Phase 4-5 tests still passing
✅ Boot sequence reaches RuntimeState::Ready successfully
✅ All 7 boot subsystems initialize without errors
✅ Device enumeration discovers expected devices
✅ Scheduler extensions functional for task management
✅ Memory pressure callbacks triggered on allocation
✅ VFS permissions enforced on inode access
✅ Integration tests pass on both x86_64 and aarch64
✅ Boot diagnostics show reasonable stage timings
✅ Zero regressions in existing functionality

## Estimated Effort

- Phase 6A (Core): 200-300 LOC, 2-3 days
- Phase 6B (Platform): 150-200 LOC, 1-2 days
- Phase 6C (Devices): 400-600 LOC, 3-4 days
- Phase 6D-F (Subsystems): 300-500 LOC, 2-3 days
- Phase 6G-H (Testing): 200-400 LOC tests, 2-3 days
- **Total**: ~1200-2200 LOC, 10-15 days estimated

## Notes

- Phase 6 is primarily integration work, not new functionality
- All trait contracts are defined; just need wiring
- Should maintain 100% of existing test coverage
- Performance tuning can be deferred to Phase 7
- Full hardware validation deferred to Phase 8+
