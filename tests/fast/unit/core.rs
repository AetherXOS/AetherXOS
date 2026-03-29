use crate::harness::{TestResult, TestCategory};
use core::sync::atomic::{AtomicUsize, Ordering};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_spinlock_basic,
        &test_spinlock_reentrant,
        &test_mutex_basic,
        &test_mutex_value_preservation,
        &test_rwlock_read,
        &test_rwlock_write,
        &test_atomic_basic,
        &test_atomic_cas,
        &test_atomic_fetch_ops,
        &test_arc_basic,
        &test_arc_clone,
        &test_arc_thread_safety,
    ]
}

fn test_spinlock_basic() -> TestResult {
    use spin::Mutex;
    let mutex = Mutex::new(42);
    let guard = mutex.lock();
    if *guard == 42 {
        TestResult::pass("core::spinlock_basic")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::spinlock_basic", "Spinlock value mismatch")
            .with_category(TestCategory::Unit)
    }
}

fn test_spinlock_reentrant() -> TestResult {
    use spin::Mutex;
    let mutex = Mutex::new(0);
    
    {
        let mut guard = mutex.lock();
        *guard = 1;
    }
    
    {
        let mut guard = mutex.lock();
        *guard = 2;
    }
    
    if *mutex.lock() == 2 {
        TestResult::pass("core::spinlock_reentrant")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::spinlock_reentrant", "Spinlock reentrant write failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_mutex_basic() -> TestResult {
    use spin::Mutex;
    let mutex = Mutex::new(100);
    let mut guard = mutex.lock();
    *guard = 200;
    drop(guard);
    
    if *mutex.lock() == 200 {
        TestResult::pass("core::mutex_basic")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::mutex_basic", "Mutex value mismatch after release")
            .with_category(TestCategory::Unit)
    }
}

fn test_mutex_value_preservation() -> TestResult {
    use spin::Mutex;
    let mutex = Mutex::new(0);
    
    for i in 0..10 {
        let mut guard = mutex.lock();
        *guard = i;
        drop(guard);
    }
    
    if *mutex.lock() == 9 {
        TestResult::pass("core::mutex_value_preservation")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::mutex_value_preservation", "Mutex value not preserved")
            .with_category(TestCategory::Unit)
    }
}

fn test_rwlock_read() -> TestResult {
    use spin::RwLock;
    let rwlock = RwLock::new(42);
    
    let read_guard1 = rwlock.read();
    let read_guard2 = rwlock.read();
    
    if *read_guard1 == 42 && *read_guard2 == 42 {
        TestResult::pass("core::rwlock_read")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::rwlock_read", "RwLock read value mismatch")
            .with_category(TestCategory::Unit)
    }
}

fn test_rwlock_write() -> TestResult {
    use spin::RwLock;
    let rwlock = RwLock::new(0);
    
    {
        let mut write_guard = rwlock.write();
        *write_guard = 100;
    }
    
    if *rwlock.read() == 100 {
        TestResult::pass("core::rwlock_write")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::rwlock_write", "RwLock write value mismatch")
            .with_category(TestCategory::Unit)
    }
}

fn test_atomic_basic() -> TestResult {
    let atomic = AtomicUsize::new(0);
    atomic.fetch_add(1, Ordering::SeqCst);
    atomic.fetch_add(2, Ordering::SeqCst);
    
    if atomic.load(Ordering::SeqCst) == 3 {
        TestResult::pass("core::atomic_basic")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::atomic_basic", "Atomic value mismatch")
            .with_category(TestCategory::Unit)
    }
}

fn test_atomic_cas() -> TestResult {
    let atomic = AtomicUsize::new(10);
    
    let success = atomic.compare_exchange(10, 20, Ordering::SeqCst, Ordering::SeqCst).is_ok();
    let value = atomic.load(Ordering::SeqCst);
    
    if success && value == 20 {
        TestResult::pass("core::atomic_cas")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::atomic_cas", "Atomic CAS failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_atomic_fetch_ops() -> TestResult {
    let atomic = AtomicUsize::new(10);
    
    atomic.fetch_and(0b1111, Ordering::SeqCst);
    let and_result = atomic.load(Ordering::SeqCst);
    
    atomic.fetch_or(0b10000, Ordering::SeqCst);
    let or_result = atomic.load(Ordering::SeqCst);
    
    atomic.fetch_xor(0b10001, Ordering::SeqCst);
    let xor_result = atomic.load(Ordering::SeqCst);
    
    if and_result == 10 && or_result == 26 && xor_result == 16 {
        TestResult::pass("core::atomic_fetch_ops")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::atomic_fetch_ops", "Atomic fetch ops failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_arc_basic() -> TestResult {
    use alloc::sync::Arc;
    use spin::Mutex;
    
    let arc = Arc::new(Mutex::new(42));
    let arc_clone = Arc::clone(&arc);
    
    if *arc_clone.lock() == 42 {
        TestResult::pass("core::arc_basic")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::arc_basic", "Arc value mismatch")
            .with_category(TestCategory::Unit)
    }
}

fn test_arc_clone() -> TestResult {
    use alloc::sync::Arc;
    
    let arc: Arc<u32> = Arc::new(100);
    let clone1 = Arc::clone(&arc);
    let clone2 = Arc::clone(&arc);
    
    if Arc::strong_count(&arc) == 3 && *clone1 == 100 && *clone2 == 100 {
        TestResult::pass("core::arc_clone")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::arc_clone", "Arc clone count or value mismatch")
            .with_category(TestCategory::Unit)
    }
}

fn test_arc_thread_safety() -> TestResult {
    use alloc::sync::Arc;
    use core::sync::atomic::AtomicUsize;
    
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    
    counter_clone.fetch_add(1, Ordering::SeqCst);
    counter.fetch_add(1, Ordering::SeqCst);
    
    if counter.load(Ordering::SeqCst) == 2 {
        TestResult::pass("core::arc_thread_safety")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("core::arc_thread_safety", "Arc thread safety test failed")
            .with_category(TestCategory::Unit)
    }
}
