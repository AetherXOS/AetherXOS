use core::sync::atomic::{AtomicUsize, Ordering};

pub fn assert_eq<T: PartialEq + core::fmt::Debug>(left: T, right: T, msg: &'static str) -> Result<(), &'static str> {
    if left != right {
        Err(msg)
    } else {
        Ok(())
    }
}

pub fn assert_ne<T: PartialEq + core::fmt::Debug>(left: T, right: T, msg: &'static str) -> Result<(), &'static str> {
    if left == right {
        Err(msg)
    } else {
        Ok(())
    }
}

pub fn assert_true(cond: bool, msg: &'static str) -> Result<(), &'static str> {
    if !cond {
        Err(msg)
    } else {
        Ok(())
    }
}

pub fn assert_false(cond: bool, msg: &'static str) -> Result<(), &'static str> {
    if cond {
        Err(msg)
    } else {
        Ok(())
    }
}

pub fn assert_ok<T, E: core::fmt::Debug>(result: Result<T, E>, msg: &'static str) -> Result<T, &'static str> {
    result.map_err(|_| msg)
}

pub fn assert_err<T, E: core::fmt::Debug>(result: Result<T, E>, msg: &'static str) -> Result<E, &'static str> {
    result.map_err(|_| msg)
}

pub fn assert_panics<F: FnOnce() + core::panic::UnwindSafe>(f: F) -> bool {
    let result = core::panic::catch_unwind(f);
    result.is_err()
}

pub fn benchmark_iterations<F: Fn()>(iterations: usize, f: F) -> u64 {
    let start = unsafe { core::arch::x86_64::_rdtsc() } as u64;
    for _ in 0..iterations {
        f();
    }
    let end = unsafe { core::arch::x86_64::_rdtsc() } as u64;
    end.saturating_sub(start) / iterations as u64
}

pub fn benchmark_ns<F: FnOnce()>(f: F) -> u64 {
    let start = unsafe { core::arch::x86_64::_rdtsc() } as u64;
    f();
    let end = unsafe { core::arch::x86_64::_rdtsc() } as u64;
    end.saturating_sub(start)
}

pub fn memory_barrier() {
    core::sync::atomic::fence(Ordering::SeqCst);
}

pub fn cpu_pause() {
    unsafe { core::arch::x86_64::_mm_pause() };
}
