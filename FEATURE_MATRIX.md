# Feature Matrix

This matrix maps Cargo features to their primary impact areas in the AetherXOS kernel.

| Feature Area | Cargo Features | Impacted Modules | Status |
|---|---|---|---|
| **Scheduler** | `sched_cfs`, `sched_round_robin`, `sched_edf`, `sched_lottery` | `kernel/src/services/scheduler` | Production |
| **Memory** | `paging_enable`, `allocator_lockfree_slab`, `allocator_buddy` | `kernel/src/services/memory` | Production |
| **IPC** | `ipc_lockfree_ring`, `ipc_futex`, `ipc_sysv_sem`, `ipc_sysv_msg` | `kernel/src/services/compat/ipc.rs` | **P0 - Gaps** |
| **VFS** | `vfs`, `posix_fs`, `vfs_ramfs`, `vfs_disk_fs` | `kernel/src/services/vfs` | **P0 - Gaps** |
| **Network** | `networking`, `posix_net`, `network_transport` | `kernel/src/services/compat/syscalls/linux_shim/net` | **P0 - Gaps** |
| **Security** | `security_capabilities`, `capability_system`, `policy_enforcement` | `kernel/src/services/security` | Production |
| **HAL** | `smp` | `kernel/src/hal` | **P1 - Gaps** |
| **Compliance**| `linux_compat` | `kernel/src/services/compat` | Mixed |

## Critical Path Status (P0/P1)

- **IPC Stubs**: Currently mapped to fixed values or ENOSYS.
- **MMU Page Table Walk**: Mock implementation in `table_walk.rs`.
- **VFS Path Resolution**: Root inode mock in `aether_fs.rs`.
- **Socket Remote Address**: Placeholder in `lifecycle.rs`.
- **CPU ID**: Placeholder in `x86_64_platform.rs`.
