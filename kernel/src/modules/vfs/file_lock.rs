/// File locking — advisory flock() and fcntl() F_SETLK / F_GETLK support.
///
/// Implements both POSIX record locks (byte-range, tied to process) and
/// BSD flock locks (whole-file, tied to file description).
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;

/// Lock types matching POSIX constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// F_RDLCK — shared / read lock.
    Read,
    /// F_WRLCK — exclusive / write lock.
    Write,
    /// F_UNLCK — unlock.
    Unlock,
}

/// A POSIX byte-range lock (fcntl F_SETLK).
#[derive(Debug, Clone)]
pub struct PosixLock {
    pub lock_type: LockType,
    /// Owning process id.
    pub pid: usize,
    /// Start offset (0 = beginning of file).
    pub start: u64,
    /// Length in bytes (0 = until EOF).
    pub len: u64,
}

impl PosixLock {
    fn end(&self) -> u64 {
        if self.len == 0 {
            u64::MAX
        } else {
            self.start + self.len
        }
    }

    fn overlaps(&self, other: &PosixLock) -> bool {
        self.start < other.end() && other.start < self.end()
    }

    fn conflicts(&self, other: &PosixLock) -> bool {
        if self.pid == other.pid {
            return false; // same process can upgrade/downgrade
        }
        if self.lock_type == LockType::Read && other.lock_type == LockType::Read {
            return false; // shared reads don't conflict
        }
        self.overlaps(other)
    }
}

/// A BSD flock lock (whole-file).
#[derive(Debug, Clone, Copy)]
pub struct FlockLock {
    pub lock_type: LockType,
    /// Owning file descriptor identity (kernel fd id, not user fd number).
    pub fd_id: usize,
}

/// Per-inode lock state.
#[derive(Debug, Default)]
struct InodeLocks {
    /// POSIX record locks on this inode.
    posix: Vec<PosixLock>,
    /// BSD flock locks on this inode.
    flocks: Vec<FlockLock>,
}

/// Global file lock manager.
pub struct LockManager {
    /// inode number → lock state.
    locks: BTreeMap<u64, InodeLocks>,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
        }
    }

    // ── POSIX fcntl locks ────────────────────────────────────

    /// Try to place a POSIX lock. Returns `Ok(())` if granted, `Err(conflicting_pid)` if blocked.
    pub fn fcntl_setlk(&mut self, ino: u64, lock: PosixLock) -> Result<(), usize> {
        let entry = self.locks.entry(ino).or_default();

        if lock.lock_type == LockType::Unlock {
            // Remove matching locks from this pid in the range.
            entry
                .posix
                .retain(|existing| !(existing.pid == lock.pid && lock.overlaps(existing)));
            self.gc(ino);
            return Ok(());
        }

        // Check for conflicts.
        for existing in &entry.posix {
            if existing.conflicts(&lock) {
                return Err(existing.pid);
            }
        }

        // Remove any existing locks from the same pid that overlap (upgrade/coalesce).
        entry
            .posix
            .retain(|existing| !(existing.pid == lock.pid && lock.overlaps(existing)));

        entry.posix.push(lock);
        Ok(())
    }

    /// Query conflicting lock (F_GETLK). Returns the conflicting lock or Unlock if none.
    pub fn fcntl_getlk(&self, ino: u64, query: &PosixLock) -> PosixLock {
        if let Some(entry) = self.locks.get(&ino) {
            for existing in &entry.posix {
                if existing.conflicts(query) {
                    return existing.clone();
                }
            }
        }
        PosixLock {
            lock_type: LockType::Unlock,
            pid: 0,
            start: query.start,
            len: query.len,
        }
    }

    /// Release all POSIX locks held by a process on an inode (called on close()).
    pub fn release_posix(&mut self, ino: u64, pid: usize) {
        if let Some(entry) = self.locks.get_mut(&ino) {
            entry.posix.retain(|l| l.pid != pid);
        }
        self.gc(ino);
    }

    // ── BSD flock locks ──────────────────────────────────────

    /// Try to place a flock lock. Returns `Ok(())` if granted, `Err` if blocked.
    pub fn flock(&mut self, ino: u64, lock: FlockLock) -> Result<(), &'static str> {
        let entry = self.locks.entry(ino).or_default();

        if lock.lock_type == LockType::Unlock {
            entry.flocks.retain(|f| f.fd_id != lock.fd_id);
            self.gc(ino);
            return Ok(());
        }

        // Check conflicts.
        for existing in &entry.flocks {
            if existing.fd_id == lock.fd_id {
                continue; // upgrading own lock
            }
            if lock.lock_type == LockType::Write || existing.lock_type == LockType::Write {
                return Err("flock: conflicting lock held");
            }
        }

        // Remove any previous lock from this fd_id (upgrade).
        entry.flocks.retain(|f| f.fd_id != lock.fd_id);
        entry.flocks.push(lock);
        Ok(())
    }

    /// Release flock lock for a file descriptor (called on close()).
    pub fn release_flock(&mut self, ino: u64, fd_id: usize) {
        if let Some(entry) = self.locks.get_mut(&ino) {
            entry.flocks.retain(|f| f.fd_id != fd_id);
        }
        self.gc(ino);
    }

    // ── Process cleanup ──────────────────────────────────────

    /// Release all locks held by a process across all inodes (called on process exit).
    pub fn release_all_for_process(&mut self, pid: usize) {
        let inos: Vec<u64> = self.locks.keys().copied().collect();
        for ino in inos {
            if let Some(entry) = self.locks.get_mut(&ino) {
                entry.posix.retain(|l| l.pid != pid);
            }
        }
        // GC empty entries.
        self.locks
            .retain(|_, v| !v.posix.is_empty() || !v.flocks.is_empty());
    }

    /// Remove empty inodes from the map.
    fn gc(&mut self, ino: u64) {
        if let Some(entry) = self.locks.get(&ino) {
            if entry.posix.is_empty() && entry.flocks.is_empty() {
                self.locks.remove(&ino);
            }
        }
    }
}

/// Global lock manager instance.
static GLOBAL_LOCK_MANAGER: Mutex<Option<LockManager>> = Mutex::new(None);

/// Initialize the global lock manager (call once at boot).
pub fn init_lock_manager() {
    let mut guard = GLOBAL_LOCK_MANAGER.lock();
    if guard.is_none() {
        *guard = Some(LockManager::new());
    }
}

/// Perform a POSIX fcntl lock operation.
pub fn fcntl_setlk(ino: u64, lock: PosixLock) -> Result<(), usize> {
    let mut guard = GLOBAL_LOCK_MANAGER.lock();
    guard.as_mut().ok_or(0usize)?.fcntl_setlk(ino, lock)
}

/// Query a POSIX lock.
pub fn fcntl_getlk(ino: u64, query: &PosixLock) -> Option<PosixLock> {
    let guard = GLOBAL_LOCK_MANAGER.lock();
    guard.as_ref().map(|mgr| mgr.fcntl_getlk(ino, query))
}

/// Place a BSD flock lock.
pub fn flock(ino: u64, lock: FlockLock) -> Result<(), &'static str> {
    let mut guard = GLOBAL_LOCK_MANAGER.lock();
    guard
        .as_mut()
        .ok_or("lock manager not initialized")?
        .flock(ino, lock)
}

/// Release all locks for a process (on process exit).
pub fn release_all_for_process(pid: usize) {
    let mut guard = GLOBAL_LOCK_MANAGER.lock();
    if let Some(ref mut mgr) = *guard {
        mgr.release_all_for_process(pid);
    }
}
