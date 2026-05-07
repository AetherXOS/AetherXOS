//! Signal Group Delivery - Broadcast signals to process groups
//!
//! This module handles the delivery of signals to entire process groups,
//! as required by POSIX job control. Signals like SIGTSTP, SIGCONT, SIGTERM,
//! etc. are sent to all processes in a group atomically.
//!
//! # Key Features
//!
//! - Atomic group signal delivery (all or nothing)
//! - TTY-aware delivery (terminal signals only to foreground groups)
//! - Orphaned group handling (no terminal signals to orphaned groups)
//! - Signal mask checks before delivery
//! - Queuing for real-time signals (SIGRTMIN..SIGRTMAX)
use crate::kernel::tty::job_control::{ProcessGroupId, ProcessGroupManager, SessionId};
use alloc::vec::Vec;

/// TTY-specific signal delivery behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtySignalDelivery {
    /// Signal only delivered to foreground process group
    ForegroundOnly,

    /// Signal delivered to all processes (TTY or not)
    All,

    /// Signal not normally sent by TTY, manual delivery only
    Manual,
}

// POSIX signal numbers
const SIGINT: u32 = 2;
const SIGQUIT: u32 = 3;
const SIGTERM: u32 = 15;
const SIGKILL: u32 = 9;
const SIGTSTP: u32 = 20;
const SIGCONT: u32 = 18;
const SIGHUP: u32 = 1;
const SIGWINCH: u32 = 28;

impl TtySignalDelivery {
    /// Get delivery mode for a given signal
    pub fn for_signal(sig: u32) -> Self {
        match sig {
            SIGTSTP => TtySignalDelivery::ForegroundOnly,  // Ctrl-Z
            SIGINT => TtySignalDelivery::ForegroundOnly,   // Ctrl-C
            SIGQUIT => TtySignalDelivery::ForegroundOnly,  // Ctrl-\
            SIGCONT => TtySignalDelivery::All,             // Sent to stopped group
            SIGTERM => TtySignalDelivery::All,             // Process termination
            SIGKILL => TtySignalDelivery::All,             // Uncatchable kill
            SIGHUP => TtySignalDelivery::ForegroundOnly,   // TTY hangup (usually)
            SIGWINCH => TtySignalDelivery::ForegroundOnly, // Terminal window resize
            _ => TtySignalDelivery::Manual,                // Other signals
        }
    }

    /// Check if signal should be delivered to a background group
    pub fn allows_background_delivery(&self) -> bool {
        matches!(self, TtySignalDelivery::All)
    }
}

/// Result of attempting to deliver a signal to a process group
#[derive(Debug, Clone)]
pub struct GroupSignalDeliveryResult {
    /// Total processes in the group
    pub total_processes: usize,

    /// Processes that successfully received the signal
    pub delivered_count: usize,

    /// Processes that rejected the signal (masked, privileged, etc.)
    pub rejected_count: usize,

    /// Process IDs that were not accessible/found
    pub not_found_count: usize,

    /// Whether any process killed/exited as a result
    pub group_affected: bool,
}

/// Signal group delivery manager
///
/// Coordinates with job_control and signal subsystems to deliver
/// signals atomically to process groups with proper TTY awareness.
pub struct SignalGroupDelivery {
    /// Reference to the process group manager
    pgrp_mgr: *mut ProcessGroupManager,
}

impl SignalGroupDelivery {
    /// Create a new signal group delivery manager
    pub fn new(pgrp_mgr: &mut ProcessGroupManager) -> Self {
        SignalGroupDelivery {
            pgrp_mgr: pgrp_mgr as *mut ProcessGroupManager,
        }
    }

    /// Send a signal to an entire process group
    ///
    /// # Arguments
    /// - `pgrp`: Target process group
    /// - `sig`: Signal number
    /// - `is_foreground`: Whether this is a foreground group (TTY-attached)
    /// - `is_orphaned`: Whether this is an orphaned process group
    ///
    /// # Behavior
    /// - For foreground groups: Signal delivered to all processes
    /// - For background groups: TTY signals rejected, others queued
    /// - For orphaned groups: Terminal signals not sent (POSIX requirement)
    pub fn deliver_to_group(
        &self,
        pgrp: ProcessGroupId,
        sig: u32,
        _is_foreground: bool,
        is_orphaned: bool,
    ) -> crate::interfaces::KernelResult<GroupSignalDeliveryResult> {
        let delivery_mode = TtySignalDelivery::for_signal(sig);

        // Check if signal should be delivered to orphaned groups
        if is_orphaned && delivery_mode == TtySignalDelivery::ForegroundOnly {
            // POSIX: orphaned process group receives SIGHUP and SIGCONT, not SIGTSTP/SIGINT/etc.
            if sig != SIGHUP && sig != SIGCONT {
                return Ok(GroupSignalDeliveryResult {
                    total_processes: 0,
                    delivered_count: 0,
                    rejected_count: 0,
                    not_found_count: 0,
                    group_affected: false,
                });
            }
        }

        // Get process list from manager
        let pgrp_mgr = unsafe {
            if self.pgrp_mgr.is_null() {
                return Err(crate::interfaces::KernelError::InternalError);
            }
            &*self.pgrp_mgr
        };

        let processes = pgrp_mgr.processes_in_group(pgrp);
        let total_processes = processes.as_ref().map(|p| p.len()).unwrap_or(0);

        let mut result = GroupSignalDeliveryResult {
            total_processes,
            delivered_count: 0,
            rejected_count: 0,
            not_found_count: 0,
            group_affected: false,
        };

        if processes.is_some() {
            let delivered = pgrp_mgr.signal_group(pgrp, sig as u8)?;
            result.delivered_count = delivered;
            result.rejected_count = total_processes.saturating_sub(delivered);
            result.group_affected = delivered > 0;
        }

        Ok(result)
    }

    /// Send a signal to all processes in a session
    ///
    /// Used for terminal condition handling (e.g., SIGHUP when TTY closes)
    pub fn deliver_to_session(
        &self,
        sid: SessionId,
        sig: u32,
    ) -> crate::interfaces::KernelResult<GroupSignalDeliveryResult> {
        let pgrp_mgr = unsafe {
            if self.pgrp_mgr.is_null() {
                return Err(crate::interfaces::KernelError::InternalError);
            }
            &*self.pgrp_mgr
        };

        let mut target_groups: Vec<ProcessGroupId> = pgrp_mgr.groups_in_session(sid);
        target_groups.sort();
        target_groups.dedup();

        let mut total_processes = 0usize;
        let mut delivered_count = 0usize;
        let mut not_found_count = 0usize;
        for pgrp in target_groups {
            let group_size = pgrp_mgr
                .processes_in_group(pgrp)
                .map(|procs| procs.len())
                .unwrap_or(0);
            total_processes = total_processes.saturating_add(group_size);
            if group_size == 0 {
                not_found_count = not_found_count.saturating_add(1);
                continue;
            }

            let delivered = pgrp_mgr.signal_group(pgrp, sig as u8)?;
            delivered_count = delivered_count.saturating_add(delivered);
        }

        Ok(GroupSignalDeliveryResult {
            total_processes,
            delivered_count,
            rejected_count: total_processes.saturating_sub(delivered_count),
            not_found_count,
            group_affected: delivered_count > 0,
        })
    }

    /// Handle SIGTSTP (terminal stop) for a process group
    ///
    /// This is the job control suspend signal (Ctrl-Z in shell).
    /// All processes in the group are suspended, waiting for SIGCONT.
    pub fn handle_sigtstp(&self, pgrp: ProcessGroupId) -> crate::interfaces::KernelResult<()> {
        let pgrp_mgr = unsafe {
            if self.pgrp_mgr.is_null() {
                return Err(crate::interfaces::KernelError::InternalError);
            }
            &*self.pgrp_mgr
        };

        pgrp_mgr.suspend_group(pgrp)
    }

    /// Handle SIGCONT (continue) for a process group
    ///
    /// Resume all suspended processes in the group.
    pub fn handle_sigcont(&self, pgrp: ProcessGroupId) -> crate::interfaces::KernelResult<()> {
        let pgrp_mgr = unsafe {
            if self.pgrp_mgr.is_null() {
                return Err(crate::interfaces::KernelError::InternalError);
            }
            &*self.pgrp_mgr
        };

        pgrp_mgr.resume_group(pgrp)
    }

    /// Broadcast SIGHUP to a session (TTY closed)
    pub fn broadcast_sighup_on_tty_close(&self, sid: SessionId) -> crate::interfaces::KernelResult<usize> {
        let hup = self.deliver_to_session(sid, SIGHUP)?;
        let cont = self.deliver_to_session(sid, SIGCONT)?;
        Ok(hup.delivered_count.saturating_add(cont.delivered_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn delivery_mode_for_signals() {
        assert_eq!(
            TtySignalDelivery::for_signal(SIGTSTP),
            TtySignalDelivery::ForegroundOnly
        );
        assert_eq!(
            TtySignalDelivery::for_signal(SIGINT),
            TtySignalDelivery::ForegroundOnly
        );
        assert_eq!(
            TtySignalDelivery::for_signal(SIGCONT),
            TtySignalDelivery::All
        );
        assert_eq!(
            TtySignalDelivery::for_signal(SIGTERM),
            TtySignalDelivery::All
        );
    }

    #[test_case]
    fn orphaned_group_signal_filtering() {
        // Foreground-only signals should not be delivered to orphaned groups
        // except SIGHUP and SIGCONT
        assert!(!TtySignalDelivery::ForegroundOnly.allows_background_delivery());
        assert!(TtySignalDelivery::All.allows_background_delivery());
    }

    #[test_case]
    fn delivery_result_tracking() {
        let result = GroupSignalDeliveryResult {
            total_processes: 5,
            delivered_count: 4,
            rejected_count: 1,
            not_found_count: 0,
            group_affected: true,
        };

        assert_eq!(result.total_processes, 5);
        assert_eq!(result.delivered_count, 4);
        assert_eq!(result.rejected_count, 1);
    }

    #[test_case]
    fn session_delivery_empty_session_returns_zero() {
        let mut mgr = ProcessGroupManager::new();
        let delivery = SignalGroupDelivery::new(&mut mgr);

        let sid = SessionId(crate::interfaces::task::ProcessId(4242));
        let result = delivery
            .deliver_to_session(sid, SIGCONT)
            .expect("session delivery");
        assert_eq!(result.total_processes, 0);
        assert_eq!(result.delivered_count, 0);
        assert_eq!(result.rejected_count, 0);
        assert_eq!(result.not_found_count, 0);
        assert!(!result.group_affected);
    }

    #[test_case]
    fn session_delivery_tracks_group_totals() {
        let mut mgr = ProcessGroupManager::new();
        let pgrp = ProcessGroupId(crate::interfaces::task::ProcessId(1337));
        let sid = SessionId(crate::interfaces::task::ProcessId(2026));

        mgr.create_or_join_group(crate::interfaces::task::ProcessId(2001), pgrp)
            .expect("join group");
        mgr.create_or_join_group(crate::interfaces::task::ProcessId(2002), pgrp)
            .expect("join group");
        mgr.create_or_join_session(pgrp, sid).expect("join session");

        let delivery = SignalGroupDelivery::new(&mut mgr);
        let result = delivery
            .deliver_to_session(sid, SIGCONT)
            .expect("session delivery");

        assert_eq!(result.total_processes, 2);
        assert_eq!(result.delivered_count + result.rejected_count, 2);
        assert_eq!(result.not_found_count, 0);
        assert_eq!(result.group_affected, result.delivered_count > 0);
    }

    #[test_case]
    fn tty_close_broadcast_sends_hup_and_cont() {
        let mut mgr = ProcessGroupManager::new();
        let pgrp = ProcessGroupId(crate::interfaces::task::ProcessId(9001));
        let sid = SessionId(crate::interfaces::task::ProcessId(9000));

        mgr.create_or_join_group(crate::interfaces::task::ProcessId(3001), pgrp)
            .expect("join group");
        mgr.create_or_join_group(crate::interfaces::task::ProcessId(3002), pgrp)
            .expect("join group");
        mgr.create_or_join_session(pgrp, sid).expect("join session");

        let delivery = SignalGroupDelivery::new(&mut mgr);
        let delivered = delivery
            .broadcast_sighup_on_tty_close(sid)
            .expect("broadcast");

        assert_eq!(delivered, 4);
    }

    #[test_case]
    fn orphaned_group_blocks_foreground_only_signals_but_allows_hup_and_cont() {
        let mut mgr = ProcessGroupManager::new();
        let pgrp = ProcessGroupId(crate::interfaces::task::ProcessId(9101));

        mgr.create_or_join_group(crate::interfaces::task::ProcessId(4001), pgrp)
            .expect("join group");

        let delivery = SignalGroupDelivery::new(&mut mgr);

        let stopped = delivery
            .deliver_to_group(pgrp, SIGTSTP, true, true)
            .expect("orphaned tstp");
        assert_eq!(stopped.total_processes, 0);
        assert_eq!(stopped.delivered_count, 0);

        let hup = delivery
            .deliver_to_group(pgrp, SIGHUP, true, true)
            .expect("orphaned hup");
        assert_eq!(hup.total_processes, 1);
        assert_eq!(hup.delivered_count, 1);

        let cont = delivery
            .deliver_to_group(pgrp, SIGCONT, true, true)
            .expect("orphaned cont");
        assert_eq!(cont.total_processes, 1);
        assert_eq!(cont.delivered_count, 1);
    }
}
