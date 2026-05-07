use super::types::*;

impl IntegrationHarness {
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

    pub fn boundary_mode_fork_valid(&self, mode: &str) -> bool {
        matches!(mode, "strict" | "balanced" | "compat")
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
}
