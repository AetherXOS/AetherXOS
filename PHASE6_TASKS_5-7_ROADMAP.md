# Phase 6 Continuation: Tasks 5-7 Roadmap

## Current Status (End of Checkpoint 2)

✅ **Phase 6 A-B Complete**: Boot manager and runtime extensions fully integrated
- 710 LOC integration code
- 45 comprehensive tests (all passing)
- Boot sequence: 6 stages orchestrated
- Runtime extensions: 29 API functions ready
- 0 compilation errors

## What's Next (Phase 6 C)

### Task 5: Device Enumeration (PRIORITY 1)

**Objective**: Replace device enumeration stubs with real ACPI/DTB parsing

#### 5.1 ACPI Parser (x86_64)
**File**: Create `kernel/src/hal/acpi_parser.rs`

```rust
pub struct AcpiParser {
    rsdp: &'static RsdpDescriptor,
}

impl AcpiParser {
    pub fn new() -> Result<Self, &'static str>;
    pub fn parse_devices(&self) -> Result<Vec<AcpiDevice>, &'static str>;
}
```

**Tasks**:
1. Parse RSDP from bootloader-provided address
2. Parse XSDT/RSDT tables
3. Extract MADT for CPU enumeration
4. Extract PCI resources from DSDT
5. Register devices with GLOBAL_DEVICE_MANAGER

#### 5.2 Device Tree Parser (aarch64)
**File**: Create `kernel/src/hal/dtb_parser.rs`

```rust
pub struct DtbParser {
    dtb_base: *const u8,
}

impl DtbParser {
    pub fn new(dtb_base: usize) -> Result<Self, &'static str>;
    pub fn parse_devices(&self) -> Result<Vec<DtbDevice>, &'static str>;
}
```

**Tasks**:
1. Parse device tree blob (FDT) format
2. Extract CPU nodes
3. Extract UART/timer/interrupt controller nodes
4. Register devices with GLOBAL_DEVICE_MANAGER

#### 5.3 Integration
Modify `boot_integration.rs::enumerate_devices()`:
```rust
pub fn enumerate_devices() -> Result<(), &'static str> {
    #[cfg(target_arch = "x86_64")]
    {
        let acpi = AcpiParser::new()?;
        let devices = acpi.parse_devices()?;
        for device in devices {
            GLOBAL_DEVICE_MANAGER.register(device.device_type, &device.name)?;
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        let dtb = DtbParser::new(bootloader_provided_dtb_base())?;
        let devices = dtb.parse_devices()?;
        for device in devices {
            GLOBAL_DEVICE_MANAGER.register(device.device_type, &device.name)?;
        }
    }
    
    Ok(())
}
```

**Estimated Effort**: 400-600 LOC, 2-3 days

---

### Task 6: Boot Subsystems Real Initialization (PRIORITY 2)

**Objective**: Replace stub implementations with actual subsystem init code

#### 6.1 Update AllocatorBootSubsystem
**File**: Modify `kernel/src/kernel/boot_subsystems.rs`

Current (stub):
```rust
fn init(&self) -> BootResult<()> {
    log::info("Initializing memory allocator subsystem");
    log::debug("Memory allocator ready");
    Ok(())
}
```

New (real):
```rust
fn init(&self) -> BootResult<()> {
    log::info("Initializing memory allocator subsystem");
    
    // Call actual allocator initialization
    allocator::init_global_allocator()?;
    allocator::enable_statistics()?;
    
    // Verify functionality
    let test_alloc = alloc::vec![1, 2, 3];
    if test_alloc.len() != 3 {
        return Err("Allocator test allocation failed");
    }
    
    log::debug("Memory allocator ready");
    Ok(())
}
```

#### 6.2 Update Other Subsystems

**SchedulerBootSubsystem**:
```rust
fn init(&self) -> BootResult<()> {
    log::info("Initializing scheduler subsystem");
    
    scheduler::init_global_scheduler()?;
    scheduler::enable_preemption()?;
    
    log::debug("Scheduler ready");
    Ok(())
}
```

**VfsBootSubsystem**:
```rust
fn init(&self) -> BootResult<()> {
    log::info("Initializing VFS subsystem");
    
    vfs::init_root_filesystem()?;
    vfs::register_filesystem_types()?;
    
    log::debug("VFS ready");
    Ok(())
}
```

Similar for: IpcBootSubsystem, InterruptBootSubsystem, SecurityBootSubsystem, ProcessBootSubsystem

**Estimated Effort**: 300-400 LOC, 2-3 days

---

### Task 7: Syscall Path Integration (PRIORITY 3)

**Objective**: Hook integration APIs into actual syscall paths

#### 7.1 Task Spawning Integration
**File**: Modify `kernel/src/kernel/task/mod.rs`

```rust
pub fn spawn_task(task: Arc<IrqSafeMutex<KernelTask>>) -> TaskId {
    let id = task.lock().id;
    register_task_arc(task.clone());
    
    // ← Phase 6 Integration
    crate::kernel_runtime::scheduler_integration::init_task_scheduler(id).ok();
    
    wake_task(id);
    id
}
```

#### 7.2 Memory Allocation Integration  
**File**: Modify page allocator (likely in `kernel/src/modules/allocators/`)

```rust
pub fn allocate_pages(pid: u32, pages: u64) -> Result<PageList, AllocError> {
    // ← Phase 6 Integration
    crate::kernel_runtime::memory_integration::record_process_allocation(pid, pages * PAGE_SIZE)?;
    
    // ... actual allocation code ...
    
    Ok(page_list)
}
```

#### 7.3 VFS Integration
**File**: Modify `kernel/src/modules/vfs/`

```rust
pub fn check_permission(inode: u64, uid: u32, gid: u32, action: u8) -> Result<(), VfsError> {
    // ← Phase 6 Integration
    crate::kernel_runtime::vfs_integration::check_file_permission(inode, uid, gid, action)
        .map_err(|_| VfsError::PermissionDenied)
}
```

**Estimated Effort**: 200-300 LOC scattered across syscall handlers, 2-3 days

---

## Implementation Sequence

**Recommended order** (respects dependencies):

1. **Task 5 (ACPI/DTB)**: Must complete before boot sequence can enumerate real devices
2. **Task 6 (Real Init)**: Can proceed in parallel with Task 5
3. **Task 7 (Syscall Integration)**: Should follow Task 6 to use initialized subsystems

**Parallel Timeline**:
- Day 1: Task 5 (ACPI) + Task 6 (Subsystem init)
- Day 2: Task 5 (DTB) + Task 6 (Remaining subsystems)
- Day 3: Task 7 (Syscall integration) + testing

---

## Files to Create

### New Files
- `kernel/src/hal/acpi_parser.rs` (200-300 LOC)
- `kernel/src/hal/dtb_parser.rs` (200-300 LOC)

### Files to Modify
- `kernel/src/kernel/boot_subsystems.rs` (update init methods)
- `kernel/src/kernel_runtime/boot_integration.rs` (expand enumerate_devices)
- `kernel/src/kernel/task/mod.rs` (add scheduler integration call)
- `kernel/src/modules/allocators/*` (add memory integration calls)
- `kernel/src/modules/vfs/*` (add permission checks)
- Various syscall handler files

---

## Testing Strategy

### Unit Tests for Task 5-6
```rust
#[test]
fn test_acpi_parsing() { /* ... */ }

#[test]
fn test_dtb_parsing() { /* ... */ }

#[test]
fn test_subsystem_initialization_order() { /* ... */ }

#[test]
fn test_device_registration() { /* ... */ }
```

### Integration Tests for Task 7
```rust
#[test]
fn test_task_spawn_with_scheduler() { /* ... */ }

#[test]
fn test_allocation_with_quota() { /* ... */ }

#[test]
fn test_permission_check_on_open() { /* ... */ }
```

### End-to-End Boot Tests
```rust
#[test]
fn test_full_boot_sequence_with_enumeration() { /* ... */ }

#[test]
fn test_first_userspace_task_creation() { /* ... */ }
```

---

## Validation Checklist - Phase 6 Complete

- [ ] ACPI parser implementation (x86_64)
- [ ] DTB parser implementation (aarch64)
- [ ] Real device enumeration
- [ ] AllocatorBootSubsystem real init
- [ ] SchedulerBootSubsystem real init
- [ ] VfsBootSubsystem real init
- [ ] IPC/Interrupt/Security/Process real init
- [ ] Task spawn syscall integration
- [ ] Memory allocation syscall integration
- [ ] VFS permission syscall integration
- [ ] 20+ new unit tests
- [ ] Integration tests passing
- [ ] QEMU boot testing (x86_64)
- [ ] QEMU boot testing (aarch64)
- [ ] Performance profiling
- [ ] Boot diagnostics reporting

---

## Known Issues to Address

1. **ACPI Parsing Complexity**: ACPI tables are complex; consider using well-tested library or precompiled data
2. **DTB Portability**: Device tree layout varies by platform; may need multiple parsers
3. **Real Init Dependencies**: Some subsystems depend on others; order matters
4. **Error Recovery**: Failed device init shouldn't block boot; graceful fallback needed
5. **Platform-Specific Code**: Much of this is architecture-specific; keep generic core clean

---

## Success Criteria for Phase 6 Complete

1. ✅ Boot manager integrated (Phase 6 A-B)
2. ⏳ Real devices enumerated (Phase 6 C - Task 5)
3. ⏳ Subsystems fully initialized (Phase 6 C - Task 6)
4. ⏳ Syscalls integrated with extensions (Phase 6 C - Task 7)
5. ⏳ Boots to UserspaceReady on real hardware
6. ⏳ All extension APIs called from runtime paths
7. ⏳ 60+ new tests covering all paths
8. ⏳ Performance profiling shows < 200ms to UserspaceReady

---

## Next Immediate Action

**Continue with Task 5: Device Enumeration**

Start by:
1. Researching ACPI table parsing (x86_64)
2. Creating kernel/src/hal/acpi_parser.rs skeleton
3. Implementing RSD pointer parsing
4. Adding to Phase 6 Branch

**Estimated time to Checkpoint 3**: 5-7 days with parallel execution
