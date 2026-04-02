use crate::interfaces::task::ProcessId;
use crate::kernel::sync::{IrqSafeMutex, WaitQueue};
use crate::kernel::task::wake_tasks;
use super::common::{suspend_on, wake_one_task};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

/// Professional Binder IPC Implementation.
///
/// Real Binder architecture:
/// 1. Processes have a 'BinderContext' containing nodes and handles.
/// 2. 'Nodes' are objects exported by a process.
/// 3. 'Handles' are local integers in a process referring to Nodes in another process.
/// 4. Transactions carry data and 'flat bind objects' (handles/nodes).

#[derive(Debug, Clone)]
pub struct BinderTransaction {
    pub sender: ProcessId,
    pub data: Vec<u8>,
    pub code: u32,
    pub flags: u32,
}

pub struct BinderNode {
    pub owner: ProcessId,
    pub ptr: u64, // User-space pointer to the object in the owner's address space
    pub cookie: u64,
}

pub struct BinderContext {
    pub pid: ProcessId,
    /// Nodes exported by THIS process to the rest of the world.
    pub nodes: Mutex<BTreeMap<u64, Arc<BinderNode>>>,
    /// Handles in THIS process referring to nodes in OTHER processes.
    pub handles: Mutex<BTreeMap<u32, Arc<BinderNode>>>,
    /// Incoming transactions to be handled by this process.
    pub todo: WaitQueue,
    pub incoming: Mutex<alloc::collections::VecDeque<BinderTransaction>>,
}

lazy_static! {
    static ref BINDER_CONTEXTS: IrqSafeMutex<BTreeMap<ProcessId, Arc<BinderContext>>> =
        IrqSafeMutex::new(BTreeMap::new());
}

pub fn get_context(pid: ProcessId) -> Arc<BinderContext> {
    let mut map = BINDER_CONTEXTS.lock();
    map.entry(pid)
        .or_insert_with(|| {
            Arc::new(BinderContext {
                pid,
                nodes: Mutex::new(BTreeMap::new()),
                handles: Mutex::new(BTreeMap::new()),
                todo: WaitQueue::new(),
                incoming: Mutex::new(alloc::collections::VecDeque::new()),
            })
        })
        .clone()
}

/// 'transact' — the core synchronous Binder call.
pub fn binder_transact(
    target_handle: u32,
    code: u32,
    data: &[u8],
    flags: u32,
) -> Result<(), &'static str> {
    let self_pid = unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| ProcessId(crate::modules::posix::process::getpid()))
            .ok_or("no process context")?
    };

    let self_ctx = get_context(self_pid);

    // 1. Locate the target node via the local handle.
    let target_node = {
        let handles = self_ctx.handles.lock();
        handles
            .get(&target_handle)
            .cloned()
            .ok_or("handle not found")?
    };

    // 2. Identify target process context.
    let target_ctx = get_context(target_node.owner);

    // 3. Deliver transaction to target.
    let tx = BinderTransaction {
        sender: self_pid,
        data: data.to_vec(),
        code,
        flags,
    };

    {
        let mut q = target_ctx.incoming.lock();
        q.push_back(tx);
    }

    // 4. Wake target process's binder threads.
    wake_one_task(&target_ctx.todo);

    Ok(())
}

/// 'read_transaction' — used by binder threads to wait for work.
pub fn binder_read(out: &mut Option<BinderTransaction>) {
    let self_pid = ProcessId(crate::modules::posix::process::getpid());
    let self_ctx = get_context(self_pid);

    loop {
        {
            let mut q = self_ctx.incoming.lock();
            if let Some(tx) = q.pop_front() {
                *out = Some(tx);
                return;
            }
        }

        // Wait for incoming work
        suspend_on(&self_ctx.todo);
    }
}

/// Compatibility stubs for the old stats interface
pub struct BinderStats {
    pub active_processes: usize,
}

pub fn binder_stats() -> BinderStats {
    BinderStats {
        active_processes: BINDER_CONTEXTS.lock().len(),
    }
}
