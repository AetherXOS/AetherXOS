//! Job Control & Process Group Management
//!
//! Implements POSIX job control for TTY-attached processes:
//! - Process groups (setpgrp, getpgrp, setpgid)
//! - Sessions (setsid, getsid)
//! - Foreground/background process group tracking
//! - Signal delivery to process groups (SIGTSTP, SIGCONT, SIGTERM)
//!
//! # Job Control State Machine
//!
//! ```text
//! Process created (inherited pgrp/sid from parent)
//!            ↓
//! [Active in Process Group] ↔ [Orphaned Process Group]
//!            ↓                          ↓
//!    [Foreground (TTY)]          [Background (TTY)]
//!            ↓
//!       SIGTSTP received
//!            ↓
//!    [Stopped/Suspended]
//!            ↓
//!       SIGCONT received
//!            ↓
//!    [Resumed/Active]
//! ```

use crate::interfaces::task::ProcessId;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU8, Ordering};

/// Process Group ID (equivalent to process ID of group leader)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessGroupId(pub ProcessId);

/// Session ID (equivalent to process ID of session leader)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub ProcessId);

/// Job control state for a process group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JobControlStateType {
    /// Process group is actively running (foreground or not attached to TTY)
    Active,

    /// Process group is suspended/stopped (waiting for SIGCONT)
    Stopped,

    /// Process group has been sent SIGCONT but some processes may still be resuming
    Resuming,

    /// Process group is background-attached to TTY (may receive SIGTSTP/SIGTTIN/SIGTTOU)
    Background,

    /// Process group's parent has exited, becoming an "orphaned process group"
    /// TTY-related signals are not delivered to orphaned groups
    Orphaned,
}

impl_enum_u8_default_conversions!(JobControlStateType {
    Active,
    Stopped,
    Resuming,
    Background,
    Orphaned,
}, default = Active);

/// Job control state container tracking per-process-group state
#[derive(Debug)]
pub struct JobControlState {
    /// Current state of the process group
    state: AtomicU8,

    /// Count of processes currently stopped in this group
    stopped_count: core::sync::atomic::AtomicUsize,

    /// Timestamp of last state change (for debugging/diagnostics)
    #[allow(dead_code)]
    state_change_ticks: core::sync::atomic::AtomicU64,
}

impl JobControlState {
    pub fn new() -> Self {
        JobControlState {
            state: AtomicU8::new(JobControlStateType::Active.to_u8()),
            stopped_count: core::sync::atomic::AtomicUsize::new(0),
            state_change_ticks: core::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Get current state
    pub fn state(&self) -> JobControlStateType {
        let byte = self.state.load(Ordering::Acquire);
        JobControlStateType::from_u8(byte).expect("invalid job control state")
    }

    /// Set the process group state
    pub fn set_state(&self, new_state: JobControlStateType) {
        self.state.store(new_state.to_u8(), Ordering::Release);
    }

    /// Increment the stopped process count
    pub fn increment_stopped_count(&self) {
        self.stopped_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Decrement the stopped process count
    pub fn decrement_stopped_count(&self) {
        self.stopped_count.fetch_sub(1, Ordering::AcqRel);
    }

    /// Get the number of stopped processes in this group
    pub fn stopped_count(&self) -> usize {
        self.stopped_count.load(Ordering::Acquire)
    }

    /// Set all processes as stopped
    pub fn mark_all_stopped(&self, total_count: usize) {
        self.stopped_count.store(total_count, Ordering::Release);
        self.set_state(JobControlStateType::Stopped);
    }

    /// Resume all processes (transition from Stopped → Active)
    pub fn mark_all_resumed(&self) {
        self.stopped_count.store(0, Ordering::Release);
        self.set_state(JobControlStateType::Active);
    }
}

/// Process Group Manager - tracks all process groups and their relationships
///
/// This is the central orchestrator for job control, managing:
/// - Process group membership
/// - Session membership
/// - Foreground/background transitions
/// - Signal delivery to groups
#[derive(Debug)]
#[allow(dead_code)]
pub struct ProcessGroupManager {
    /// All active process groups: pgrp_id → set of process IDs
    groups: BTreeMap<ProcessGroupId, alloc::vec::Vec<ProcessId>>,

    /// All active sessions: sid → set of process group IDs
    sessions: BTreeMap<SessionId, alloc::vec::Vec<ProcessGroupId>>,

    /// Job control state per process group
    group_states: BTreeMap<ProcessGroupId, JobControlState>,
}

#[allow(dead_code)]
impl ProcessGroupManager {
    pub fn new() -> Self {
        ProcessGroupManager {
            groups: BTreeMap::new(),
            sessions: BTreeMap::new(),
            group_states: BTreeMap::new(),
        }
    }

    /// Create or join a process group
    pub fn create_or_join_group(
        &mut self,
        pid: ProcessId,
        pgrp: ProcessGroupId,
    ) -> crate::interfaces::KernelResult<()> {
        self.groups.entry(pgrp).or_insert_with(alloc::vec::Vec::new).push(pid);
        self.group_states.entry(pgrp).or_insert_with(JobControlState::new);
        Ok(())
    }

    /// Create or join a session
    pub fn create_or_join_session(
        &mut self,
        pgrp: ProcessGroupId,
        sid: SessionId,
    ) -> crate::interfaces::KernelResult<()> {
        self.sessions.entry(sid).or_insert_with(alloc::vec::Vec::new).push(pgrp);
        Ok(())
    }

    /// Get the current state of a process group
    pub fn group_state(&self, pgrp: ProcessGroupId) -> Option<JobControlStateType> {
        self.group_states.get(&pgrp).map(|state| state.state())
    }

    /// Get all processes in a process group
    pub fn processes_in_group(&self, pgrp: ProcessGroupId) -> Option<alloc::vec::Vec<ProcessId>> {
        self.groups.get(&pgrp).cloned()
    }

    /// Remove a process from its group
    pub fn remove_process(&mut self, pid: ProcessId, pgrp: ProcessGroupId) {
        if let Some(procs) = self.groups.get_mut(&pgrp) {
            procs.retain(|&p| p != pid);
            if procs.is_empty() {
                self.groups.remove(&pgrp);
                self.group_states.remove(&pgrp);
            }
        }
    }

    /// Check if a process group is orphaned
    /// (all parent processes in the session have exited)
    pub fn is_orphaned(&self, _pgrp: ProcessGroupId) -> bool {
        // TODO: Implement orphan detection logic
        // An orphaned process group has no parent process that is member of the same session
        false
    }

    /// Deliver a signal to all processes in a group
    pub fn signal_group(
        &self,
        pgrp: ProcessGroupId,
        _signal: u8,
    ) -> crate::interfaces::KernelResult<usize> {
        if let Some(procs) = self.groups.get(&pgrp) {
            let mut delivered = 0;
            for &_pid in procs {
                // TODO: Actually deliver signal to process
                // For now, just count
                delivered += 1;
            }
            return Ok(delivered);
        }
        Ok(0)
    }

    /// Handle SIGTSTP (suspend) for a process group
    pub fn suspend_group(&self, pgrp: ProcessGroupId) -> crate::interfaces::KernelResult<()> {
        if let Some(state) = self.group_states.get(&pgrp) {
            let proc_count = self.groups.get(&pgrp).map(|p| p.len()).unwrap_or(0);
            state.mark_all_stopped(proc_count);
        }
        Ok(())
    }

    /// Handle SIGCONT (resume) for a process group
    pub fn resume_group(&self, pgrp: ProcessGroupId) -> crate::interfaces::KernelResult<()> {
        if let Some(state) = self.group_states.get(&pgrp) {
            state.mark_all_resumed();
        }
        Ok(())
    }

    /// Mark a process group as background
    pub fn mark_background(&self, pgrp: ProcessGroupId) -> crate::interfaces::KernelResult<()> {
        if let Some(state) = self.group_states.get(&pgrp) {
            state.set_state(JobControlStateType::Background);
        }
        Ok(())
    }

    /// Mark a process group as foreground
    pub fn mark_foreground(&self, pgrp: ProcessGroupId) -> crate::interfaces::KernelResult<()> {
        if let Some(state) = self.group_states.get(&pgrp) {
            state.set_state(JobControlStateType::Active);
        }
        Ok(())
    }

    /// Mark a process group as orphaned
    pub fn mark_orphaned(&self, pgrp: ProcessGroupId) -> crate::interfaces::KernelResult<()> {
        if let Some(state) = self.group_states.get(&pgrp) {
            state.set_state(JobControlStateType::Orphaned);
        }
        Ok(())
    }
}

impl Default for ProcessGroupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn job_control_state_transitions() {
        let state = JobControlState::new();
        assert_eq!(state.state(), JobControlStateType::Active);

        state.set_state(JobControlStateType::Stopped);
        assert_eq!(state.state(), JobControlStateType::Stopped);

        state.set_state(JobControlStateType::Active);
        assert_eq!(state.state(), JobControlStateType::Active);
    }

    #[test_case]
    fn stopped_process_counting() {
        let state = JobControlState::new();
        assert_eq!(state.stopped_count(), 0);

        state.increment_stopped_count();
        state.increment_stopped_count();
        assert_eq!(state.stopped_count(), 2);

        state.decrement_stopped_count();
        assert_eq!(state.stopped_count(), 1);
    }

    #[test_case]
    fn process_group_manager_creation() {
        let mut mgr = ProcessGroupManager::new();
        let pid = ProcessId(1001);
        let pgrp = ProcessGroupId(ProcessId(1100));

        mgr.create_or_join_group(pid, pgrp).unwrap();
        let procs = mgr.processes_in_group(pgrp);
        assert!(procs.is_some());
        assert_eq!(procs.unwrap()[0], pid);
    }

    #[test_case]
    fn process_group_suspend_resume() {
        let mut mgr = ProcessGroupManager::new();
        let pgrp = ProcessGroupId(ProcessId(1100));

        mgr.create_or_join_group(ProcessId(1001), pgrp).unwrap();
        mgr.create_or_join_group(ProcessId(1002), pgrp).unwrap();

        mgr.suspend_group(pgrp).unwrap();
        assert_eq!(mgr.group_state(pgrp), Some(JobControlStateType::Stopped));

        mgr.resume_group(pgrp).unwrap();
        assert_eq!(mgr.group_state(pgrp), Some(JobControlStateType::Active));
    }

    #[test_case]
    fn process_removal_from_group() {
        let mut mgr = ProcessGroupManager::new();
        let pgrp = ProcessGroupId(ProcessId(1100));

        mgr.create_or_join_group(ProcessId(1001), pgrp).unwrap();
        mgr.create_or_join_group(ProcessId(1002), pgrp).unwrap();

        let procs = mgr.processes_in_group(pgrp);
        assert_eq!(procs.unwrap().len(), 2);

        mgr.remove_process(ProcessId(1001), pgrp);
        let procs = mgr.processes_in_group(pgrp);
        assert_eq!(procs.unwrap().len(), 1);
    }

    #[test_case]
    fn empty_group_cleanup() {
        let mut mgr = ProcessGroupManager::new();
        let pgrp = ProcessGroupId(ProcessId(1100));

        mgr.create_or_join_group(ProcessId(1001), pgrp).unwrap();
        mgr.remove_process(ProcessId(1001), pgrp);

        let procs = mgr.processes_in_group(pgrp);
        assert!(procs.is_none()); // Group should be removed when empty
    }

    #[test_case]
    fn session_creation() {
        let mut mgr = ProcessGroupManager::new();
        let pgrp = ProcessGroupId(ProcessId(1100));
        let sid = SessionId(ProcessId(1100));

        mgr.create_or_join_session(pgrp, sid).unwrap();
        // Sessions are valid if they don't error
    }
}
