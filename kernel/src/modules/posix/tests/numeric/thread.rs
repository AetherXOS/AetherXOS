use super::*;
use crate::modules::posix::{thread, ipc};

#[test_case]
#[cfg(all(feature = "posix_thread", feature = "posix_ipc"))]
fn thread_library_mutex_and_condvar_work() {
    let me = thread::pthread_self();
    assert!(thread::pthread_equal(me, me));
    thread::sched_yield();

    let mutex = thread::PthreadMutex::new(0xAA11);
    let cond = thread::PthreadCondvar::new(0xAA22);

    assert!(mutex.try_lock().expect("try_lock") );
    assert!(!mutex.try_lock().expect("try_lock contended"));
    mutex.unlock().expect("unlock");

    mutex.lock().expect("lock");
    cond.wait(&mutex).expect("cond wait");
    assert!(ipc::futex_pending_waiters(cond.key()) >= 1);
    let woke = cond.signal().expect("cond signal");
    assert!(woke >= 1);
    mutex.unlock().expect("unlock after cond wait");
}

#[test_case]
#[cfg(feature = "posix_thread")]
fn thread_library_semaphore_and_rwlock_work() {
    let sem = thread::PosixSemaphore::new(1, 0xBB11);
    assert!(sem.try_wait().expect("sem try_wait first"));
    assert!(!sem.try_wait().expect("sem try_wait empty"));
    sem.post().expect("sem post");
    sem.wait().expect("sem wait");

    let rw = thread::PthreadRwLock::new(0xBB22);
    rw.rdlock().expect("rw rdlock");
    rw.unlock().expect("rw runlock");
    rw.wrlock().expect("rw wrlock");
    rw.unlock().expect("rw wunlock");
}

#[test_case]
#[cfg(feature = "posix_thread")]
fn thread_lifecycle_helpers_behave_consistently() {
    let me = thread::pthread_self();
    if me != 0 {
        assert!(thread::thread_exists(me));
        assert_eq!(thread::pthread_join(me, 2), Err(PosixErrno::Invalid));
        thread::pthread_detach(me).expect("detach self");
        assert_eq!(thread::pthread_join(me, 2), Err(PosixErrno::Invalid));
    }

    let synthetic = me.saturating_add(10_000);
    thread::pthread_register(synthetic).expect("register synthetic");
    assert!(thread::thread_exists(synthetic));
    thread::pthread_detach(synthetic).expect("detach synthetic");

    assert_eq!(thread::pthread_join(usize::MAX, 0), Err(PosixErrno::Invalid));
}

#[test_case]
#[cfg(feature = "posix_thread")]
fn thread_create_from_image_validates_inputs() {
    assert_eq!(thread::pthread_register(0), Err(PosixErrno::Invalid));
    let result = thread::pthread_create_from_image(b"", b"", 10, 0, 0, 0);
    #[cfg(feature = "process_abstraction")]
    assert_eq!(result, Err(PosixErrno::Invalid));
    #[cfg(not(feature = "process_abstraction"))]
    assert_eq!(result, Err(PosixErrno::NotSupported));
}
