pub mod cpu;
pub mod freezer;
pub mod io;
/// Cgroups v2 Framework — Unified resource control hierarchy.
///
/// Implements Linux cgroups v2 semantics with pluggable controllers:
/// - **memory**: RSS/swap limits, usage tracking, OOM control
/// - **cpu**: Bandwidth quota (`cpu.max`), weight-based sharing
/// - **io**: Throttle per-device I/O bandwidth
/// - **pids**: Limit number of tasks in a cgroup
/// - **freezer**: Suspend/resume all tasks in a cgroup
///
/// ## Architecture
///
/// The cgroup tree is a hierarchical directory-like structure.
/// Each cgroup node can have controllers enabled via `cgroup.subtree_control`.
/// Resource constraints are inherited: a child cannot exceed its parent's limits.
pub mod memory;
pub mod pids;

use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;

pub use self::memory::MemoryController;
pub use cpu::CpuController;
pub use freezer::FreezerController;
pub use io::IoController;
pub use pids::PidsController;

// ─── Telemetry ───────────────────────────────────────────────────────

static CG_CREATES: AtomicU64 = AtomicU64::new(0);
static CG_DESTROYS: AtomicU64 = AtomicU64::new(0);
static CG_ATTACH: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct CgroupStats {
    pub creates: u64,
    pub destroys: u64,
    pub attaches: u64,
}

pub fn cgroup_stats() -> CgroupStats {
    CgroupStats {
        creates: CG_CREATES.load(Ordering::Relaxed),
        destroys: CG_DESTROYS.load(Ordering::Relaxed),
        attaches: CG_ATTACH.load(Ordering::Relaxed),
    }
}

// ─── Controller Enum ─────────────────────────────────────────────────

bitflags::bitflags! {
    /// Set of active controllers for a cgroup subtree.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Controllers: u8 {
        const MEMORY  = 0b0000_0001;
        const CPU     = 0b0000_0010;
        const IO      = 0b0000_0100;
        const PIDS    = 0b0000_1000;
        const FREEZER = 0b0001_0000;
    }
}

// ─── Cgroup ID ───────────────────────────────────────────────────────

pub type CgroupId = u64;

static NEXT_CG_ID: AtomicU64 = AtomicU64::new(1);

fn alloc_cg_id() -> CgroupId {
    NEXT_CG_ID.fetch_add(1, Ordering::Relaxed)
}

// ─── Cgroup Node ─────────────────────────────────────────────────────

/// A single cgroup node in the hierarchy.
pub struct CgroupNode {
    pub id: CgroupId,
    pub name: String,
    pub parent_id: Option<CgroupId>,
    pub children: Vec<CgroupId>,
    /// Tasks (by global PID) attached to this cgroup.
    pub tasks: Vec<u64>,
    /// Controllers active in this subtree.
    pub subtree_control: Controllers,
    /// Per-controller state.
    pub memory_ctrl: MemoryController,
    pub cpu_ctrl: CpuController,
    pub io_ctrl: IoController,
    pub pids_ctrl: PidsController,
    pub freezer_ctrl: FreezerController,
}

impl CgroupNode {
    fn new(name: String, parent_id: Option<CgroupId>) -> Self {
        CG_CREATES.fetch_add(1, Ordering::Relaxed);
        Self {
            id: alloc_cg_id(),
            name,
            parent_id,
            children: Vec::new(),
            tasks: Vec::new(),
            subtree_control: Controllers::empty(),
            memory_ctrl: MemoryController::new(),
            cpu_ctrl: CpuController::new(),
            io_ctrl: IoController::new(),
            pids_ctrl: PidsController::new(),
            freezer_ctrl: FreezerController::new(),
        }
    }
}

// ─── Cgroup Manager ──────────────────────────────────────────────────

/// Central cgroup tree manager.
pub struct CgroupManager {
    /// All cgroup nodes, keyed by ID.
    nodes: BTreeMap<CgroupId, CgroupNode>,
    /// Path → CgroupId mapping for lookups.
    path_to_id: BTreeMap<String, CgroupId>,
    /// Root cgroup ID.
    pub root_id: CgroupId,
}

impl CgroupManager {
    /// Create a new cgroup manager with a root cgroup.
    pub fn new() -> Self {
        let mut root = CgroupNode::new(String::from("/"), None);
        root.subtree_control = Controllers::CPU | Controllers::MEMORY;
        let root_id = root.id;
        let mut nodes = BTreeMap::new();
        let mut path_to_id = BTreeMap::new();
        path_to_id.insert(String::from("/"), root_id);
        nodes.insert(root_id, root);
        Self {
            nodes,
            path_to_id,
            root_id,
        }
    }

    /// Create a child cgroup under `parent_path`.  Returns the new cgroup's ID.
    pub fn create(&mut self, parent_path: &str, name: &str) -> Option<CgroupId> {
        let parent_id = *self.path_to_id.get(parent_path)?;
        let child_path = if parent_path == "/" {
            alloc::format!("/{}", name)
        } else {
            alloc::format!("{}/{}", parent_path, name)
        };
        if self.path_to_id.contains_key(&child_path) {
            return None; // Already exists.
        }
        let child = CgroupNode::new(String::from(name), Some(parent_id));
        let child_id = child.id;
        self.nodes.insert(child_id, child);
        self.path_to_id.insert(child_path, child_id);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(child_id);
        }
        Some(child_id)
    }

    /// Destroy a cgroup. Must have no tasks and no children.
    pub fn destroy(&mut self, path: &str) -> bool {
        let id = match self.path_to_id.get(path) {
            Some(&id) => id,
            None => return false,
        };
        if id == self.root_id {
            return false; // Cannot destroy root.
        }
        if let Some(node) = self.nodes.get(&id) {
            if !node.tasks.is_empty() || !node.children.is_empty() {
                return false;
            }
            let parent_id = node.parent_id;
            self.nodes.remove(&id);
            self.path_to_id.remove(path);
            if let Some(pid) = parent_id {
                if let Some(parent) = self.nodes.get_mut(&pid) {
                    parent.children.retain(|&c| c != id);
                }
            }
            CG_DESTROYS.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Attach a task to a cgroup.
    pub fn attach_task(&mut self, path: &str, task_pid: u64) -> bool {
        // First, detach from any existing cgroup.
        for node in self.nodes.values_mut() {
            node.tasks.retain(|&t| t != task_pid);
        }
        let id = match self.path_to_id.get(path) {
            Some(&id) => id,
            None => return false,
        };
        if let Some(node) = self.nodes.get_mut(&id) {
            // Check pids limit.
            if node.pids_ctrl.max > 0 && node.tasks.len() as u64 >= node.pids_ctrl.max {
                return false;
            }
            node.tasks.push(task_pid);
            CG_ATTACH.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Detach a task from its cgroup.
    pub fn detach_task(&mut self, task_pid: u64) {
        for node in self.nodes.values_mut() {
            node.tasks.retain(|&t| t != task_pid);
        }
    }

    /// Enable controllers for a subtree.
    pub fn set_subtree_control(&mut self, path: &str, controllers: Controllers) -> bool {
        let id = match self.path_to_id.get(path) {
            Some(&id) => id,
            None => return false,
        };
        if let Some(node) = self.nodes.get_mut(&id) {
            node.subtree_control = controllers;
            true
        } else {
            false
        }
    }

    /// Get a reference to a cgroup node by path.
    pub fn get(&self, path: &str) -> Option<&CgroupNode> {
        let id = self.path_to_id.get(path)?;
        self.nodes.get(id)
    }

    /// Get a mutable reference to a cgroup node by path.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut CgroupNode> {
        let id = *self.path_to_id.get(path)?;
        self.nodes.get_mut(&id)
    }

    /// Find which cgroup a task belongs to.
    pub fn find_task_cgroup(&self, task_pid: u64) -> Option<&str> {
        for (path, &id) in &self.path_to_id {
            if let Some(node) = self.nodes.get(&id) {
                if node.tasks.contains(&task_pid) {
                    return Some(path.as_str());
                }
            }
        }
        None
    }

    /// List all cgroup paths.
    pub fn list_paths(&self) -> Vec<&str> {
        self.path_to_id.keys().map(|s| s.as_str()).collect()
    }

    /// Freeze all tasks in a cgroup.
    pub fn freeze(&mut self, path: &str) -> bool {
        if let Some(node) = self.get_mut(path) {
            node.freezer_ctrl.freeze();
            true
        } else {
            false
        }
    }

    /// Thaw all tasks in a cgroup.
    pub fn thaw(&mut self, path: &str) -> bool {
        if let Some(node) = self.get_mut(path) {
            node.freezer_ctrl.thaw();
            true
        } else {
            false
        }
    }

    /// Charge CPU time (microseconds) to cgroup `id`.
    /// Returns `false` if the CPU controller is active and the cgroup is throttled.
    pub fn charge_cpu_by_id(&mut self, id: CgroupId, us: u64) -> bool {
        if let Some(node) = self.nodes.get_mut(&id) {
            if node.subtree_control.contains(Controllers::CPU) {
                return node.cpu_ctrl.try_charge(us);
            }
        }
        true // no CPU controller active ⇒ never throttled
    }

    /// Charge memory usage (bytes) to cgroup `id`.
    /// Returns `false` if the memory controller is active and the limit is exceeded.
    pub fn charge_memory_by_id(&mut self, id: CgroupId, bytes: u64) -> bool {
        if let Some(node) = self.nodes.get_mut(&id) {
            if node.subtree_control.contains(Controllers::MEMORY) {
                return node.memory_ctrl.try_charge(bytes);
            }
        }
        true
    }

    /// Uncharge memory from cgroup `id`.
    pub fn uncharge_memory_by_id(&mut self, id: CgroupId, bytes: u64) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.memory_ctrl.uncharge(bytes);
        }
    }

    /// Reset CPU period counters for all cgroups. Call once per bandwidth period.
    pub fn reset_cpu_periods(&mut self) {
        for node in self.nodes.values_mut() {
            node.cpu_ctrl.reset_period();
        }
    }
}

// ─── Well-known cgroup IDs ────────────────────────────────────────────

/// The root cgroup always receives ID 1 (first call to `alloc_cg_id()`).
pub const ROOT_CGROUP_ID: CgroupId = 1;

// ─── Global singleton ────────────────────────────────────────────────

lazy_static! {
    /// Kernel-wide cgroup manager.  All resource charge calls go through this.
    pub static ref CGROUP_MANAGER: IrqSafeMutex<CgroupManager> =
        IrqSafeMutex::new(CgroupManager::new());
}

// ─── Public charge API ───────────────────────────────────────────────

/// Charge CPU time (µs) to cgroup `id`.  Returns `false` if throttled.
#[inline]
pub fn cgroup_charge_cpu(id: CgroupId, us: u64) -> bool {
    CGROUP_MANAGER.lock().charge_cpu_by_id(id, us)
}

/// Charge memory (bytes) to cgroup `id`.  Returns `false` if limit exceeded.
#[inline]
pub fn cgroup_charge_memory(id: CgroupId, bytes: u64) -> bool {
    CGROUP_MANAGER.lock().charge_memory_by_id(id, bytes)
}

/// Uncharge memory (bytes) from cgroup `id`.
#[inline]
pub fn cgroup_uncharge_memory(id: CgroupId, bytes: u64) {
    CGROUP_MANAGER.lock().uncharge_memory_by_id(id, bytes);
}

/// Reset all CPU period counters.  Should be called once per bandwidth period (~100 ms).
#[inline]
pub fn cgroup_reset_all_periods() {
    CGROUP_MANAGER.lock().reset_cpu_periods();
}
