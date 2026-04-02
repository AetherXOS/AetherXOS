#[path = "process_runtime.rs"]
mod process_runtime;
#[path = "process_types.rs"]
mod process_types;
#[cfg(feature = "posix_mman")]
#[path = "process_vdso.rs"]
mod process_vdso;
use crate::interfaces::security::{ResourceLimits, SecurityLevel};
use crate::interfaces::task::{ProcessId, TaskId};
use crate::kernel::sync::IrqSafeMutex;
#[cfg(feature = "vfs")]
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use process_runtime as runtime_ops;
pub use process_runtime::bind_prepared_image_snapshot;
pub use process_types::{
    MappingRecord, ProcessLifecycleState, ProcessRuntimeContractSnapshot, RuntimeLifecycleHooks,
};
pub(crate) use crate::kernel::memory::{PAGE_ALIGN_MASK, PAGE_SIZE_BYTES_U64};
#[cfg(feature = "posix_mman")]
use process_vdso::{build_minimal_vdso_page, build_minimal_vvar_page};
#[cfg(feature = "paging_enable")]
use x86_64::PhysAddr;

const PROCESS_NAME_LEN: usize = 32;
const PROCESS_THREAD_INLINE_CAPACITY: usize = 4;

impl ProcessId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
        ProcessId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A Process is a container for resources (Memory, File Handles).
/// It does NOT execute code; Threads (Tasks) do.
pub struct Process {
    pub id: ProcessId,
    pub name: IrqSafeMutex<[u8; PROCESS_NAME_LEN]>,

    // Page Table Physical Address (Only if Paging is enabled)
    #[cfg(feature = "paging_enable")]
    pub cr3: PhysAddr,

    pub threads: IrqSafeMutex<Vec<TaskId>>,

    pub image_entry: AtomicUsize,
    pub runtime_entry: AtomicU64,
    pub runtime_fini_entry: AtomicU64,
    pub image_base: AtomicU64,
    pub image_pages: AtomicUsize,
    pub image_segments: AtomicUsize,
    pub tls_mem_size: AtomicU64,
    pub tls_align: AtomicU64,
    pub image_phdr_addr: AtomicU64,
    pub image_phent_size: AtomicU32,
    pub image_phnum: AtomicU32,
    pub vdso_base: AtomicU64,
    pub vvar_base: AtomicU64,
    pub vdso_map_id: AtomicU32,
    pub vvar_map_id: AtomicU32,
    pub exec_generation: AtomicU64,
    pub mapped_regions: AtomicUsize,
    pub mapped_pages: AtomicUsize,
    pub lifecycle_state: AtomicU8,
    pub exit_status: AtomicI32,
    pub exec_path: IrqSafeMutex<String>,
    pub tls_template: IrqSafeMutex<Vec<u8>>,
    pub runtime_hooks: IrqSafeMutex<RuntimeLifecycleHooks>,
    // Records for mmap'd regions owned by this process
    pub mappings: crate::kernel::sync::IrqSafeMutex<alloc::vec::Vec<MappingRecord>>,
    // A simple hint allocator for anonymous/file-backed mmap allocations.
    pub next_mapping_hint: AtomicU64,

    // Capability Mask: Which syscalls are allowed?
    #[cfg(feature = "capabilities")]
    pub capabilities: u64,

    /// Resource limits for this process.
    pub resource_limits: ResourceLimits,
    /// Current open file descriptor count (for rlimit enforcement).
    pub open_file_count: AtomicU32,
    /// MAC security level for this process.
    pub security_level: SecurityLevel,
    /// Namespace ID handle into kernel namespace registry (0 = root namespace).
    pub namespace_id: AtomicU32,
    /// Cgroup ID for memory/CPU resource accounting (1 = root cgroup).
    pub cgroup_id: AtomicU64,

    /// File Descriptor Table.
    #[cfg(feature = "vfs")]
    pub files:
        IrqSafeMutex<alloc::collections::BTreeMap<usize, Box<dyn crate::modules::vfs::File>>>,

    /// Signal Handlers.
    pub signal_handlers: IrqSafeMutex<alloc::collections::BTreeMap<i32, u64>>,
}

impl Process {
    #[inline(always)]
    fn build_with_thread_capacity(
        name: &[u8],
        thread_capacity: usize,
        #[cfg(feature = "paging_enable")] cr3: PhysAddr,
    ) -> Self {
        let mut name_buf = [0u8; PROCESS_NAME_LEN];
        let len = core::cmp::min(name.len(), PROCESS_NAME_LEN);
        name_buf[..len].copy_from_slice(&name[..len]);

        Self {
            id: ProcessId::new(),
            name: IrqSafeMutex::new(name_buf),
            #[cfg(feature = "paging_enable")]
            cr3,
            threads: IrqSafeMutex::new(Vec::with_capacity(thread_capacity)),
            image_entry: AtomicUsize::new(0),
            runtime_entry: AtomicU64::new(0),
            runtime_fini_entry: AtomicU64::new(0),
            image_base: AtomicU64::new(0),
            image_pages: AtomicUsize::new(0),
            image_segments: AtomicUsize::new(0),
            tls_mem_size: AtomicU64::new(0),
            tls_align: AtomicU64::new(0),
            image_phdr_addr: AtomicU64::new(0),
            image_phent_size: AtomicU32::new(0),
            image_phnum: AtomicU32::new(0),
            vdso_base: AtomicU64::new(0),
            vvar_base: AtomicU64::new(0),
            vdso_map_id: AtomicU32::new(0),
            vvar_map_id: AtomicU32::new(0),
            exec_generation: AtomicU64::new(0),
            mapped_regions: AtomicUsize::new(0),
            mapped_pages: AtomicUsize::new(0),
            lifecycle_state: AtomicU8::new(ProcessLifecycleState::Created.to_u8()),
            exit_status: AtomicI32::new(0),
            exec_path: IrqSafeMutex::new(String::new()),
            tls_template: IrqSafeMutex::new(Vec::new()),
            runtime_hooks: IrqSafeMutex::new(RuntimeLifecycleHooks::default()),
            mappings: crate::kernel::sync::IrqSafeMutex::new(alloc::vec::Vec::new()),
            next_mapping_hint: AtomicU64::new(0x0000_7000_0000_0000),
            #[cfg(feature = "capabilities")]
            capabilities: 0, // Default: No privileges
            resource_limits: ResourceLimits::default_user(),
            open_file_count: AtomicU32::new(0),
            security_level: SecurityLevel::Unclassified,
            namespace_id: AtomicU32::new(0),
            cgroup_id: AtomicU64::new(1), // root cgroup
            #[cfg(feature = "vfs")]
            files: IrqSafeMutex::new(alloc::collections::BTreeMap::new()),
            signal_handlers: IrqSafeMutex::new(alloc::collections::BTreeMap::new()),
        }
    }

    pub fn new(name: &[u8], #[cfg(feature = "paging_enable")] cr3: PhysAddr) -> Self {
        Self::build_with_thread_capacity(
            name,
            PROCESS_THREAD_INLINE_CAPACITY,
            #[cfg(feature = "paging_enable")]
            cr3,
        )
    }

    #[inline(always)]
    pub fn new_bootstrap(name: &[u8], #[cfg(feature = "paging_enable")] cr3: PhysAddr) -> Self {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap ctor begin\n",
        );
        let process = Self::build_with_thread_capacity(
            name,
            0,
            #[cfg(feature = "paging_enable")]
            cr3,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap ctor returned\n",
        );
        process
    }

    pub fn rename(&self, new_name: &[u8]) {
        let mut name = self.name.lock();
        name.fill(0);
        let len = core::cmp::min(new_name.len(), 32);
        name[..len].copy_from_slice(&new_name[..len]);
    }

    pub fn add_thread(&self, task_id: TaskId) {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] process add_thread lock begin\n");
        let mut threads = self.threads.lock();
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] process add_thread lock returned\n");
        threads.push(task_id);
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] process add_thread push returned\n");
    }

    pub fn add_thread_unpublished(&self, task_id: TaskId) {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process add_thread bootstrap borrow begin\n",
        );
        let threads = unsafe { self.threads.bootstrap_borrow_mut() };
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process add_thread bootstrap borrow returned\n",
        );
        threads.push(task_id);
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process add_thread bootstrap push returned\n",
        );
    }

    #[inline(always)]
    pub fn add_bootstrap_thread(&self, task_id: TaskId) {
        self.add_thread_unpublished(task_id);
    }

    pub fn bind_module_load_plan(
        &self,
        plan: &crate::kernel::module_loader::ModuleLoadPlan,
    ) -> Result<(), &'static str> {
        runtime_ops::bind_module_load_plan(self, plan)
    }

    pub fn image_state(&self) -> (usize, usize, usize, u64) {
        (
            self.image_entry.load(Ordering::Relaxed),
            self.image_pages.load(Ordering::Relaxed),
            self.image_segments.load(Ordering::Relaxed),
            self.exec_generation.load(Ordering::Relaxed),
        )
    }

    pub fn bind_virtual_mappings(
        &self,
        mappings: &[crate::kernel::module_loader::VirtualMappingPlan],
    ) -> Result<(), &'static str> {
        runtime_ops::bind_virtual_mappings(self, mappings)
    }

    pub fn mapping_state(&self) -> (usize, usize) {
        (
            self.mapped_regions.load(Ordering::Relaxed),
            self.mapped_pages.load(Ordering::Relaxed),
        )
    }

    /// Allocate a user-space virtual address range for an `mmap` of `len` bytes.
    /// Returns the start virtual address on success.
    pub fn allocate_user_vaddr(&self, len: usize) -> Result<u64, &'static str> {
        runtime_ops::allocate_user_vaddr(self, len)
    }

    pub fn register_mapping(
        &self,
        map_id: u32,
        start: u64,
        end: u64,
        prot: u32,
        flags: u32,
    ) -> Result<(), &'static str> {
        runtime_ops::register_mapping(self, map_id, start, end, prot, flags)
    }

    pub fn lookup_mapping(&self, vaddr: u64) -> Option<MappingRecord> {
        let m = self.mappings.lock();
        for record in m.iter() {
            if vaddr >= record.start && vaddr < record.end {
                return Some(*record);
            }
        }
        None
    }

    pub fn overlapping_mappings(&self, start: u64, end: u64) -> Vec<MappingRecord> {
        let m = self.mappings.lock();
        m.iter()
            .copied()
            .filter(|record| start < record.end && end > record.start)
            .collect()
    }

    pub fn remove_mapping(&self, map_id: u32) {
        let _ = self.remove_mapping_record(map_id);
    }

    pub fn remove_mapping_record(&self, map_id: u32) -> Option<MappingRecord> {
        runtime_ops::remove_mapping_record(self, map_id)
    }

    #[inline(always)]
    pub fn lifecycle_state(&self) -> ProcessLifecycleState {
        runtime_ops::lifecycle_state(self).expect("lifecycle state must be valid enum")
    }

    #[inline(always)]
    pub fn runtime_state(&self) -> (ProcessLifecycleState, i32, u64) {
        (
            self.lifecycle_state(),
            self.exit_status.load(Ordering::Relaxed),
            self.exec_generation.load(Ordering::Relaxed),
        )
    }

    #[inline(always)]
    pub fn auxv_state(&self) -> (usize, usize, usize, usize, usize, usize, usize) {
        runtime_ops::auxv_state(self)
    }

    #[inline(always)]
    pub fn set_exec_path(&self, path: &str) {
        runtime_ops::set_exec_path(self, path);
    }

    #[inline(always)]
    pub fn set_runtime_entry(&self, entry: Option<u64>) {
        self.runtime_entry
            .store(entry.unwrap_or(0), Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn set_runtime_fini_entry(&self, entry: Option<u64>) {
        self.runtime_fini_entry
            .store(entry.unwrap_or(0), Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn effective_entry(&self) -> usize {
        runtime_ops::effective_entry(self)
    }

    pub fn exec_path_snapshot(&self) -> String {
        self.exec_path.lock().clone()
    }

    #[cfg(feature = "posix_mman")]
    pub fn ensure_linux_runtime_mappings(&self) -> Result<(u64, u64), &'static str> {
        let page_size = crate::interfaces::memory::PAGE_SIZE_4K as usize;
        let read = crate::modules::posix_consts::mman::PROT_READ as u32;
        let exec = crate::modules::posix_consts::mman::PROT_EXEC as u32;
        let private = crate::modules::posix_consts::mman::MAP_PRIVATE as u32;

        let old_vvar = self.vvar_map_id.swap(0, Ordering::Relaxed);
        if old_vvar != 0 {
            self.remove_mapping(old_vvar);
        }
        let old_vdso = self.vdso_map_id.swap(0, Ordering::Relaxed);
        if old_vdso != 0 {
            self.remove_mapping(old_vdso);
        }

        let vvar_map_id = crate::modules::posix::mman::mmap_anonymous(page_size, read, private)
            .map_err(|_| "vvar mmap failed")?;
        let vvar_base = self.allocate_user_vaddr(page_size)?;
        self.register_mapping(
            vvar_map_id,
            vvar_base,
            vvar_base + page_size as u64,
            read,
            private,
        )?;

        let vdso_map_id =
            crate::modules::posix::mman::mmap_anonymous(page_size, read | exec, private)
                .map_err(|_| "vdso mmap failed")?;
        let vdso_base = self.allocate_user_vaddr(page_size)?;
        self.register_mapping(
            vdso_map_id,
            vdso_base,
            vdso_base + page_size as u64,
            read | exec,
            private,
        )?;

        self.vvar_map_id.store(vvar_map_id, Ordering::Relaxed);
        self.vdso_map_id.store(vdso_map_id, Ordering::Relaxed);
        self.vvar_base.store(vvar_base, Ordering::Relaxed);
        self.vdso_base.store(vdso_base, Ordering::Relaxed);
        self.populate_linux_runtime_pages(vdso_base, vvar_base, vdso_map_id, vvar_map_id)?;
        Ok((vdso_base, vvar_base))
    }

    #[cfg(feature = "posix_mman")]
    fn populate_linux_runtime_pages(
        &self,
        vdso_base: u64,
        vvar_base: u64,
        vdso_map_id: u32,
        vvar_map_id: u32,
    ) -> Result<(), &'static str> {
        let page_size = crate::interfaces::memory::PAGE_SIZE_4K as usize;
        let vdso_page = build_minimal_vdso_page(page_size, vdso_base, vvar_base);
        crate::modules::posix::mman::mmap_write(vdso_map_id, &vdso_page, 0)
            .map_err(|_| "vdso populate failed")?;

        let vvar_page =
            build_minimal_vvar_page(page_size, self.image_entry.load(Ordering::Relaxed));
        crate::modules::posix::mman::mmap_write(vvar_map_id, &vvar_page, 0)
            .map_err(|_| "vvar populate failed")?;
        Ok(())
    }

    #[cfg(feature = "posix_mman")]
    pub fn refresh_linux_runtime_vvar(&self) -> Result<(), &'static str> {
        let vvar_map_id = self.vvar_map_id.load(Ordering::Relaxed);
        if vvar_map_id == 0 {
            return Ok(());
        }
        let page_size = crate::interfaces::memory::PAGE_SIZE_4K as usize;
        let vvar_page =
            build_minimal_vvar_page(page_size, self.image_entry.load(Ordering::Relaxed));
        crate::modules::posix::mman::mmap_write(vvar_map_id, &vvar_page, 0)
            .map_err(|_| "vvar refresh failed")?;
        Ok(())
    }

    pub fn bind_tls_template(
        &self,
        image: &[u8],
        plan: &crate::kernel::module_loader::ModuleLoadPlan,
    ) -> Result<(), &'static str> {
        runtime_ops::bind_tls_template(self, image, plan)
    }

    pub fn tls_state_snapshot(&self) -> (Vec<u8>, u64, u64) {
        (
            self.tls_template.lock().clone(),
            self.tls_mem_size.load(Ordering::Relaxed),
            self.tls_align.load(Ordering::Relaxed),
        )
    }

    pub fn set_runtime_hooks(&self, hooks: RuntimeLifecycleHooks) {
        *self.runtime_hooks.lock() = hooks;
    }

    pub fn append_deferred_fini_calls(&self, fini_calls: &[u64]) {
        runtime_ops::append_deferred_fini_calls(self, fini_calls);
    }

    pub fn runtime_hooks_snapshot(&self) -> RuntimeLifecycleHooks {
        self.runtime_hooks.lock().clone()
    }

    pub fn runtime_contract_snapshot(&self) -> ProcessRuntimeContractSnapshot {
        runtime_ops::runtime_contract_snapshot(self)
    }

    pub fn clear_runtime_contract(&self) {
        runtime_ops::clear_runtime_contract(self);
    }

    #[inline(always)]
    pub fn mark_runnable(&self) {
        self.lifecycle_state
            .store(ProcessLifecycleState::Runnable.to_raw(), Ordering::Relaxed);
        self.exit_status.store(0, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn mark_running(&self) {
        self.lifecycle_state
            .store(ProcessLifecycleState::Running.to_raw(), Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn mark_exited(&self, status: i32) {
        self.exit_status.store(status, Ordering::Relaxed);
        self.lifecycle_state
            .store(ProcessLifecycleState::Exited.to_raw(), Ordering::Relaxed);
    }

    pub fn create_bootstrap_task_from_image(
        name: &[u8],
        image: &[u8],
        task_id: crate::interfaces::TaskId,
        priority: u8,
        deadline: u64,
        burst_time: u64,
        kernel_stack_top: u64,
        #[cfg(feature = "paging_enable")] cr3: PhysAddr,
    ) -> Result<
        (
            alloc::sync::Arc<Self>,
            alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
        ),
        crate::kernel::module_loader::ProcessPrepareError,
    > {
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "create_begin",
            Some(task_id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );
        let process = alloc::sync::Arc::new(Self::new_bootstrap(
            name,
            #[cfg(feature = "paging_enable")]
            cr3,
        ));
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "process_ready",
            Some(process.id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap build task begin\n",
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap loader call begin\n",
        );
        let task = crate::kernel::module_loader::build_process_bootstrap_task(
            &process,
            image,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        )?;
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap loader call returned\n",
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap build task returned\n",
        );
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "task_ready",
            Some(task_id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );

        Ok((process, task))
    }

    pub fn create_bootstrap_task_from_snapshot(
        name: &[u8],
        image: &[u8],
        snapshot: crate::kernel::module_loader::ModuleImageSnapshot,
        task_id: crate::interfaces::TaskId,
        priority: u8,
        deadline: u64,
        burst_time: u64,
        kernel_stack_top: u64,
        #[cfg(feature = "paging_enable")] cr3: PhysAddr,
    ) -> Result<
        (
            alloc::sync::Arc<Self>,
            alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
        ),
        crate::kernel::module_loader::ProcessPrepareError,
    > {
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "create_begin",
            Some(task_id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );
        let process = alloc::sync::Arc::new(Self::new_bootstrap(
            name,
            #[cfg(feature = "paging_enable")]
            cr3,
        ));
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "process_ready",
            Some(process.id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap build task begin\n",
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap loader call begin\n",
        );
        let task = crate::kernel::module_loader::build_process_bootstrap_task_from_snapshot(
            &process,
            image,
            snapshot,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        )?;
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap loader call returned\n",
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap build task returned\n",
        );
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "task_ready",
            Some(task_id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );

        Ok((process, task))
    }

    #[cfg(feature = "paging_enable")]
    pub fn materialize_bootstrap_task_from_image(
        name: &[u8],
        image: &[u8],
        task_id: crate::interfaces::TaskId,
        priority: u8,
        deadline: u64,
        burst_time: u64,
        kernel_stack_top: u64,
        cr3: PhysAddr,
        page_manager: &mut crate::kernel::memory::paging::PageManager,
        frame_allocator: &mut impl x86_64::structures::paging::FrameAllocator<
            x86_64::structures::paging::Size4KiB,
        >,
    ) -> Result<
        (
            alloc::sync::Arc<Self>,
            alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>,
        ),
        crate::kernel::module_loader::ProcessPrepareError,
    > {
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "materialized_create_begin",
            Some(task_id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );
        let process = alloc::sync::Arc::new(Self::new_bootstrap(name, cr3));
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "materialized_process_ready",
            Some(process.id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap materialized build task begin\n",
        );
        let task = crate::kernel::module_loader::materialize_and_build_process_bootstrap_task(
            &process,
            image,
            task_id,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
            page_manager,
            frame_allocator,
        )?;
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw(
            "[EARLY SERIAL] process bootstrap materialized build task returned\n",
        );
        crate::kernel::debug_trace::record_with_metadata(
            "process.bootstrap",
            "materialized_task_ready",
            Some(task_id.0 as u64),
            false,
            crate::kernel::debug_trace::TraceSeverity::Trace,
            crate::kernel::debug_trace::TraceCategory::Launch,
        );

        Ok((process, task))
    }
}
