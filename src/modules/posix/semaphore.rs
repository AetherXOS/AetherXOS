use crate::modules::posix::thread::PosixSemaphore;
use crate::modules::posix::PosixErrno;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref NAMED_SEMAPHORES: Mutex<BTreeMap<String, Arc<PosixSemaphore>>> =
        Mutex::new(BTreeMap::new());
}

pub fn sem_open(
    name: &str,
    oflag: i32,
    _mode: u32,
    value: u32,
) -> Result<Arc<PosixSemaphore>, PosixErrno> {
    if !name.starts_with('/') || name.len() < 2 {
        return Err(PosixErrno::Invalid);
    }

    let mut sems = NAMED_SEMAPHORES.lock();
    if let Some(sem) = sems.get(name) {
        if (oflag & crate::modules::posix_consts::fs::O_CREAT) != 0
            && (oflag & crate::modules::posix_consts::fs::O_EXCL) != 0
        {
            return Err(PosixErrno::AlreadyExists);
        }
        return Ok(sem.clone());
    }

    if (oflag & crate::modules::posix_consts::fs::O_CREAT) == 0 {
        return Err(PosixErrno::NoEntry);
    }

    let sem = Arc::new(PosixSemaphore::new(value, generate_sem_key(name)));
    sems.insert(String::from(name), sem.clone());
    Ok(sem)
}

pub fn sem_post(sem: &PosixSemaphore) -> Result<(), PosixErrno> {
    sem.post()
}

pub fn sem_wait(sem: &PosixSemaphore) -> Result<(), PosixErrno> {
    sem.wait()
}

pub fn sem_trywait(sem: &PosixSemaphore) -> Result<bool, PosixErrno> {
    sem.try_wait()
}

pub fn sem_unlink(name: &str) -> Result<(), PosixErrno> {
    if NAMED_SEMAPHORES.lock().remove(name).is_some() {
        Ok(())
    } else {
        Err(PosixErrno::NoEntry)
    }
}

fn generate_sem_key(name: &str) -> u64 {
    let mut hash = 0u64;
    for b in name.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(*b as u64);
    }
    // Salt it to avoid collisions with other IPC keys
    hash ^ 0x5E_BA_0000_0000_0000
}
