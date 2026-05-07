use crate::interfaces::security::{ResourceLimits, SecurityLevel};
use crate::interfaces::task::{ProcessId, TaskId};
use crate::kernel::sync::IrqSafeMutex;
#[cfg(feature = "vfs")]
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use super::types::{MappingRecord, ProcessLifecycleState, ProcessRuntimeContractSnapshot, RuntimeLifecycleHooks};
#[cfg(feature = "paging_enable")]
use x86_64::PhysAddr;

pub const PROCESS_NAME_LEN: usize = 32;

pub struct Process {
    pub id: ProcessId,
    pub name: IrqSafeMutex<[u8; PROCESS_NAME_LEN]>,

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
    pub mappings: IrqSafeMutex<Vec<MappingRecord>>,
    pub next_mapping_hint: AtomicU64,

    #[cfg(feature = "capabilities")]
    pub capabilities: u64,

    pub resource_limits: ResourceLimits,
    pub open_file_count: AtomicU32,
    pub security_level: SecurityLevel,
    pub namespace_id: AtomicU32,
    pub cgroup_id: AtomicU64,

    #[cfg(feature = "vfs")]
    pub files: IrqSafeMutex<alloc::collections::BTreeMap<usize, Box<dyn crate::modules::vfs::File>>>,

    pub signal_handlers: IrqSafeMutex<alloc::collections::BTreeMap<i32, u64>>,
    
    pub parent_id: AtomicUsize,
    pub exit_wait_queue: crate::kernel::sync::WaitQueue,
    pub interpreter_base: AtomicU64,
    pub heap_start: AtomicU64,
    pub heap_break: AtomicU64,
}



impl Process {
    pub fn new(name: &[u8]) -> Self {
        #[cfg(feature = "paging_enable")]
        let cr3 = x86_64::PhysAddr::new(0);
        
        let mut n = [0u8; PROCESS_NAME_LEN];
        let len = name.len().min(PROCESS_NAME_LEN);
        n[..len].copy_from_slice(&name[..len]);

        Self {
            id: ProcessId::new(),
            name: IrqSafeMutex::new(n),
            #[cfg(feature = "paging_enable")]
            cr3,
            threads: IrqSafeMutex::new(Vec::new()),
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
            exec_generation: AtomicU64::new(1),
            mapped_regions: AtomicUsize::new(0),
            mapped_pages: AtomicUsize::new(0),
            lifecycle_state: AtomicU8::new(ProcessLifecycleState::Created as u8),
            exit_status: AtomicI32::new(0),
            exec_path: IrqSafeMutex::new(String::new()),
            tls_template: IrqSafeMutex::new(Vec::new()),
            runtime_hooks: IrqSafeMutex::new(RuntimeLifecycleHooks::default()),
            mappings: IrqSafeMutex::new(Vec::new()),
            next_mapping_hint: AtomicU64::new(0x4000_0000_0000),
            #[cfg(feature = "capabilities")]
            capabilities: 0xFFFF_FFFF_FFFF_FFFF,
            resource_limits: ResourceLimits::default(),
            open_file_count: AtomicU32::new(0),
            security_level: SecurityLevel::Unclassified,
            namespace_id: AtomicU32::new(0),
            cgroup_id: AtomicU64::new(1),
            #[cfg(feature = "vfs")]
            files: IrqSafeMutex::new(alloc::collections::BTreeMap::new()),
            signal_handlers: IrqSafeMutex::new(alloc::collections::BTreeMap::new()),
            parent_id: AtomicUsize::new(0),
            exit_wait_queue: crate::kernel::sync::WaitQueue::new(),
            interpreter_base: AtomicU64::new(0),
            heap_start: AtomicU64::new(0),
            heap_break: AtomicU64::new(0),
        }
    }



    pub fn new_with_cr3(name: &[u8], cr3: x86_64::PhysAddr) -> Self {
        #[allow(unused_mut)]
        let mut p = Self::new(name);
        #[cfg(feature = "paging_enable")]
        {
            p.cr3 = cr3;
        }
        let _ = cr3;
        p
    }

    pub fn mark_runnable(&self) {
        self.lifecycle_state
            .store(ProcessLifecycleState::Runnable as u8, Ordering::SeqCst);
    }

    pub fn mark_running(&self) {
        self.lifecycle_state
            .store(ProcessLifecycleState::Running as u8, Ordering::SeqCst);
    }

    pub fn mark_exited(&self, status: i32) {
        self.exit_status.store(status, Ordering::SeqCst);
        self.lifecycle_state
            .store(ProcessLifecycleState::Exited as u8, Ordering::SeqCst);
        self.exit_wait_queue.wake_all();
    }


    pub fn is_exited(&self) -> bool {
        self.lifecycle_state.load(Ordering::SeqCst) == ProcessLifecycleState::Exited as u8
    }

    pub fn image_state(&self) -> (usize, usize, usize, u64) {
        (
            self.image_entry.load(Ordering::Relaxed),
            self.image_pages.load(Ordering::Relaxed),
            self.image_segments.load(Ordering::Relaxed),
            self.image_base.load(Ordering::Relaxed),
        )
    }

    pub fn mapping_state(&self) -> (usize, usize) {
        (
            self.mapped_regions.load(Ordering::Relaxed),
            self.mapped_pages.load(Ordering::Relaxed),
        )
    }

    pub fn register_mapping(
        &self,
        id: u32,
        start: u64,
        end: u64,
        prot: u32,
        flags: u32,
    ) -> Result<(), &'static str> {
        let mut mappings = self.mappings.lock();
        if mappings.iter().any(|m| m.map_id == id) {
            return Err("mapping id already exists");
        }
        mappings.push(MappingRecord {
            map_id: id,
            start,
            end,
            prot,
            flags,
        });
        self.mapped_regions.fetch_add(1, Ordering::Relaxed);
        let pages = ((end - start) as usize + 4095) / 4096;
        self.mapped_pages.fetch_add(pages, Ordering::Relaxed);
        Ok(())
    }

    pub fn overlapping_mappings(&self, start: u64, end: u64) -> Vec<MappingRecord> {
        let mappings = self.mappings.lock();
        mappings
            .iter()
            .filter(|m| m.start < end && m.end > start)
            .cloned()
            .collect()
    }

    pub fn remove_mapping(&self, id: u32) -> bool {
        self.remove_mapping_record(id).is_some()
    }

    pub fn remove_mapping_record(&self, id: u32) -> Option<MappingRecord> {
        let mut mappings = self.mappings.lock();
        if let Some(pos) = mappings.iter().position(|m| m.map_id == id) {
            let record = mappings.remove(pos);
            self.mapped_regions.fetch_sub(1, Ordering::Relaxed);
            let pages = ((record.end - record.start) as usize + 4095) / 4096;
            self.mapped_pages.fetch_sub(pages, Ordering::Relaxed);
            Some(record)
        } else {
            None
        }
    }

    pub fn lookup_mapping(&self, vaddr: u64) -> Option<MappingRecord> {
        let mappings = self.mappings.lock();
        mappings
            .iter()
            .find(|m| vaddr >= m.start && vaddr < m.end)
            .cloned()
    }

    pub fn rename(&self, name: &[u8]) {
        let mut n = self.name.lock();
        let len = name.len().min(PROCESS_NAME_LEN);
        n.fill(0);
        n[..len].copy_from_slice(&name[..len]);
    }

    pub fn runtime_contract_snapshot(&self) -> ProcessRuntimeContractSnapshot {
        let hooks = self.runtime_hooks.lock();
        ProcessRuntimeContractSnapshot {
            image_entry: self.image_entry.load(Ordering::Relaxed),
            runtime_entry: self.runtime_entry.load(Ordering::Relaxed) as usize,
            runtime_fini_entry: self.runtime_fini_entry.load(Ordering::Relaxed) as usize,
            image_base: self.image_base.load(Ordering::Relaxed) as usize,
            phdr_addr: self.image_phdr_addr.load(Ordering::Relaxed) as usize,
            vdso_base: self.vdso_base.load(Ordering::Relaxed) as usize,
            vvar_base: self.vvar_base.load(Ordering::Relaxed) as usize,
            exec_path: self.exec_path.lock().clone(),
            init_calls: hooks.ordered_init_calls(),
            fini_calls: hooks.ordered_fini_calls(),
        }
    }

    pub fn clear_runtime_contract(&self) {
        let mut hooks = self.runtime_hooks.lock();
        hooks.deferred_fini.clear();
    }

    pub fn append_deferred_fini_calls(&self, calls: &[u64]) {
        let mut hooks = self.runtime_hooks.lock();
        for &call in calls {
            if !hooks.deferred_fini.contains(&call) {
                hooks.deferred_fini.push(call);
            }
        }
    }

    pub fn add_bootstrap_thread(&self, tid: TaskId) {
        self.threads.lock().push(tid);
    }

    pub fn set_runtime_hooks(&self, hooks: RuntimeLifecycleHooks) {
        *self.runtime_hooks.lock() = hooks;
    }

    pub fn set_exec_path(&self, path: &str) {
        *self.exec_path.lock() = String::from(path);
    }

    pub fn effective_entry(&self) -> u64 {
        let runtime = self.runtime_entry.load(Ordering::Relaxed);
        if runtime != 0 {
            runtime
        } else {
            self.image_entry.load(Ordering::Relaxed) as u64
        }
    }

    pub fn runtime_hooks_snapshot(&self) -> RuntimeLifecycleHooks {
        self.runtime_hooks.lock().clone()
    }

    pub fn exec_path_snapshot(&self) -> String {
        self.exec_path.lock().clone()
    }

    pub fn tls_state_snapshot(&self) -> (u64, u64, Vec<u8>) {
        (
            self.tls_mem_size.load(Ordering::Relaxed),
            self.tls_align.load(Ordering::Relaxed),
            self.tls_template.lock().clone(),
        )
    }

    pub fn allocate_user_vaddr(&self, len: usize) -> Result<u64, &'static str> {
        super::runtime::allocate_user_vaddr(self, len)
    }

    pub fn set_runtime_entry(&self, entry: Option<u64>) {
        self.runtime_entry.store(entry.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_runtime_fini_entry(&self, entry: Option<u64>) {
        self.runtime_fini_entry.store(entry.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_interpreter_base(&self, base: u64) {
        self.interpreter_base.store(base, Ordering::Relaxed);
    }


    pub fn bind_module_load_plan(&self, plan: &crate::kernel::module_loader::ModuleLoadPlan) -> Result<(), &'static str> {
        self.image_entry.store(plan.entry as usize, Ordering::Relaxed);
        self.image_base.store(plan.aslr_base, Ordering::Relaxed);
        self.image_phdr_addr.store(plan.program_header_addr, Ordering::Relaxed);
        self.image_phent_size.store(plan.program_header_entry_size as u32, Ordering::Relaxed);
        self.image_phnum.store(plan.program_headers as u32, Ordering::Relaxed);
        self.tls_mem_size.store(plan.tls_mem_size, Ordering::Relaxed);
        self.tls_align.store(plan.tls_align, Ordering::Relaxed);

        let mut max_end = plan.aslr_base;
        for seg in &plan.segments {
            let end = seg.virtual_addr + seg.mem_size;
            if end > max_end {
                max_end = end;
            }
        }
        // Round up to page boundary
        let heap_start = (max_end + 4095) & !4095;
        self.heap_start.store(heap_start, Ordering::Relaxed);
        self.heap_break.store(heap_start, Ordering::Relaxed);

        Ok(())
    }

    pub fn auxv_state(&self) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
        (
            self.image_entry.load(Ordering::Relaxed) as u64,
            self.image_base.load(Ordering::Relaxed),
            self.image_phdr_addr.load(Ordering::Relaxed),
            self.image_phent_size.load(Ordering::Relaxed) as u64,
            self.image_phnum.load(Ordering::Relaxed) as u64,
            self.vdso_base.load(Ordering::Relaxed),
            self.vvar_base.load(Ordering::Relaxed),
            self.interpreter_base.load(Ordering::Relaxed),
        )
    }


    #[cfg(feature = "posix_mman")]
    pub fn ensure_linux_runtime_mappings(&self) -> Result<(), &'static str> {
        if self.vdso_base.load(Ordering::Relaxed) != 0 {
            return Ok(());
        }

        // Allocate VVAR (1 page)
        let vvar_size = 4096;
        let vvar_addr = super::runtime::allocate_user_vaddr(self, vvar_size)?;
        self.vvar_base.store(vvar_addr, Ordering::Relaxed);

        // Allocate vDSO (1 page)
        let vdso_size = 4096;
        let vdso_addr = super::runtime::allocate_user_vaddr(self, vdso_size)?;
        self.vdso_base.store(vdso_addr, Ordering::Relaxed);

        #[cfg(all(feature = "paging_enable", target_arch = "x86_64"))]
        {
            let vdso_phys = super::vdso::get_vdso_phys_frame();
            let vvar_phys = super::vdso::get_vvar_phys_frame();
            
            if vdso_phys != 0 && vvar_phys != 0 {
                let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
                let offset = x86_64::VirtAddr::new(hhdm);
                let lvl4 = unsafe { &mut *( (self.cr3.as_u64() + hhdm) as *mut x86_64::structures::paging::PageTable ) };
                let mut page_manager = crate::kernel::memory::paging::PageManager {
                    mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
                    physical_memory_offset: offset,
                };
                let mut frame_allocator = crate::hal::HAL::create_frame_allocator();

                use crate::interfaces::memory::page_flags as bits;
                let _vdso_flags = bits::PRESENT | bits::USER | bits::NO_EXECUTE; // vdso needs exec? Usually yes for functions
                let _vvar_flags = bits::PRESENT | bits::USER | bits::NO_EXECUTE;

                // Fixup vdso page with this process's bases
                unsafe {
                    let vdso_ptr = (vdso_phys as u64 + hhdm) as *mut u8;
                    let _buf = core::slice::from_raw_parts_mut(vdso_ptr, 4096);
                    // Note: If we share the physical page, we can't do process-specific fixups here!
                    // Linux vDSO is position-independent and doesn't need fixups in the page itself,
                    // or uses a different mechanism.
                    // For now, we'll assume the page is generic or we'll accept the lack of fixups.
                }

                let _ = page_manager.map_page(vdso_addr, vdso_phys as u64, bits::PRESENT | bits::USER, &mut frame_allocator);
                let _ = page_manager.map_page(vvar_addr, vvar_phys as u64, bits::PRESENT | bits::USER | bits::NO_EXECUTE, &mut frame_allocator);
            }
        }

        crate::klog_info!(
            "[PROCESS] ensured linux runtime mappings: vvar={:#x} vdso={:#x}",
            vvar_addr,
            vdso_addr
        );

        Ok(())
    }

    #[cfg(feature = "posix_mman")]
    pub fn refresh_linux_runtime_vvar(&self) -> Result<(), &'static str> {
        #[cfg(feature = "posix_time")]
        {
            let rt = crate::modules::posix::time::clock_gettime_raw(0).unwrap_or(crate::modules::posix::time::PosixTimespec { sec: 0, nsec: 0 });
            let mono = crate::modules::posix::time::clock_gettime_raw(1).unwrap_or(crate::modules::posix::time::PosixTimespec { sec: 0, nsec: 0 });
            super::vdso::update_vvar_time(rt.sec as u64, rt.nsec as u32, mono.sec as u64, mono.nsec as u32);
            Ok(())
        }
        #[cfg(not(feature = "posix_time"))]
        {
            Ok(())
        }
    }

    pub fn set_brk(&self, new_brk: u64) -> Result<u64, &'static str> {
        let old_brk = self.heap_break.load(Ordering::Relaxed);
        if new_brk == 0 || new_brk == old_brk {
            return Ok(old_brk);
        }

        if new_brk < self.heap_start.load(Ordering::Relaxed) {
            return Ok(old_brk);
        }

        let page_size = 4096u64;
        let old_page_end = (old_brk + page_size - 1) & !(page_size - 1);
        let new_page_end = (new_brk + page_size - 1) & !(page_size - 1);

        if new_page_end > old_page_end {
            // Growing: Map new pages
            #[cfg(feature = "paging_enable")]
            {
                let hhdm = crate::hal::hhdm_offset().unwrap_or(0);
                let offset = x86_64::VirtAddr::new(hhdm);
                let lvl4 = unsafe { &mut *( (self.cr3.as_u64() + hhdm) as *mut x86_64::structures::paging::PageTable ) };
                let mut page_manager = crate::kernel::memory::paging::PageManager {
                    mapper: unsafe { x86_64::structures::paging::OffsetPageTable::new(lvl4, offset) },
                    physical_memory_offset: offset,
                };
                let mut frame_allocator = crate::hal::HAL::create_frame_allocator();

                use crate::interfaces::memory::page_flags as bits;
                let flags = bits::PRESENT | bits::WRITABLE | bits::USER;

                let mut curr = old_page_end;
                while curr < new_page_end {
                    let frame = frame_allocator.allocate_frame().ok_or("OOM during brk")?;
                    let _ = page_manager.map_page(curr, frame.start_address().as_u64(), flags, &mut frame_allocator);
                    curr += page_size;
                }
            }
        } else if new_page_end < old_page_end {
            // Shrinking: Unmap pages (deferred - requires PageManager unmap implementation)
            // For now, we update the boundary but don't unmap to avoid complex page table surgery
            // unless we have a robust VMA system.
        }

        self.heap_break.store(new_brk, Ordering::SeqCst);
        Ok(new_brk)
    }
}
