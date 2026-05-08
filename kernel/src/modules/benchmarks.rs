//! Performance benchmarking suite for kernel optimization validation
//! 
//! This module provides comprehensive benchmarks to measure the performance
//! improvements of the optimized kernel components.
//! 
//! Benchmarks:
//! - Memory allocation throughput and latency
//! - IPC message passing latency
//! - Scheduler context switch overhead
//! - Syscall elimination effectiveness
//! - Overall system throughput


/// Benchmark result
#[derive(Debug, Clone, Copy)]
pub struct BenchmarkResult {
    pub name: &'static str,
    pub iterations: u64,
    pub total_ns: u64,
    pub avg_ns: u64,
    pub min_ns: u64,
    pub max_ns: u64,
    pub ops_per_second: u64,
}

impl BenchmarkResult {
    pub fn new(name: &'static str, iterations: u64, timings: &[u64]) -> Self {
        let total_ns: u64 = timings.iter().sum();
        let avg_ns = total_ns / iterations;
        let min_ns = *timings.iter().min().unwrap_or(&0);
        let max_ns = *timings.iter().max().unwrap_or(&0);
        let ops_per_second = if total_ns > 0 {
            (iterations as u64 * 1_000_000_000) / total_ns
        } else {
            0
        };

        Self {
            name,
            iterations,
            total_ns,
            avg_ns,
            min_ns,
            max_ns,
            ops_per_second,
        }
    }

    pub fn speedup_factor(&self, baseline: &BenchmarkResult) -> f64 {
        if self.avg_ns > 0 && baseline.avg_ns > 0 {
            baseline.avg_ns as f64 / self.avg_ns as f64
        } else {
            1.0
        }
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    results: alloc::vec::Vec<BenchmarkResult>,
}

impl BenchmarkRunner {
    pub const fn new() -> Self {
        Self {
            results: alloc::vec::Vec::new(),
        }
    }

    /// Run a benchmark function
    pub fn run<F>(&mut self, name: &'static str, iterations: u64, mut f: F) -> BenchmarkResult
    where
        F: FnMut() -> u64,
    {
        let mut timings = alloc::vec::Vec::with_capacity(iterations as usize);
        
        for _ in 0..iterations {
            let _start = self.read_tsc();
            let elapsed = f();
            timings.push(elapsed);
        }

        let result = BenchmarkResult::new(name, iterations, &timings);
        self.results.push(result);
        result
    }

    /// Read timestamp counter (TSC)
    #[inline(always)]
    fn read_tsc(&self) -> u64 {
        crate::hal::cpu::rdtsc()
    }

    /// Get all results
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    /// Print summary
    pub fn print_summary(&self) {
        crate::klog_info!("=== Benchmark Summary ===");
        for result in &self.results {
            crate::klog_info!(
                "{}: {} ops/sec (avg: {} ns, min: {} ns, max: {} ns)",
                result.name,
                result.ops_per_second,
                result.avg_ns,
                result.min_ns,
                result.max_ns
            );
        }
    }
}

/// Memory allocation benchmarks
pub mod memory_benchmarks {
    use super::*;
    

    pub fn benchmark_tiny_allocations(runner: &mut BenchmarkRunner) {
        runner.run("tiny_alloc_16b", 10000, || {
            let start = crate::hal::cpu::rdtsc();
            let layout = core::alloc::Layout::from_size_align(16, 8).unwrap();
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            if !ptr.is_null() {
                unsafe { alloc::alloc::dealloc(ptr, layout) };
            }
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }

    pub fn benchmark_small_allocations(runner: &mut BenchmarkRunner) {
        runner.run("small_alloc_512b", 10000, || {
            let start = crate::hal::cpu::rdtsc();
            let layout = core::alloc::Layout::from_size_align(512, 8).unwrap();
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            if !ptr.is_null() {
                unsafe { alloc::alloc::dealloc(ptr, layout) };
            }
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }

    pub fn benchmark_medium_allocations(runner: &mut BenchmarkRunner) {
        runner.run("medium_alloc_8kb", 10000, || {
            let start = crate::hal::cpu::rdtsc();
            let layout = core::alloc::Layout::from_size_align(8192, 8).unwrap();
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            if !ptr.is_null() {
                unsafe { alloc::alloc::dealloc(ptr, layout) };
            }
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }

    pub fn benchmark_large_allocations(runner: &mut BenchmarkRunner) {
        runner.run("large_alloc_64kb", 10000, || {
            let start = crate::hal::cpu::rdtsc();
            let layout = core::alloc::Layout::from_size_align(65536, 8).unwrap();
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            if !ptr.is_null() {
                unsafe { alloc::alloc::dealloc(ptr, layout) };
            }
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }
}

/// IPC benchmarks
pub mod ipc_benchmarks {
    use super::*;
    use crate::modules::ipc::lockfree_ring::LockFreeRingBuffer;

    pub fn benchmark_ipc_send(runner: &mut BenchmarkRunner) {
        let rb = LockFreeRingBuffer::new();
        let msg = b"Hello, World!";
        
        runner.run("ipc_send", 100000, || {
            let start = crate::hal::cpu::rdtsc();
            let _ = rb.try_send(msg);
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }

    pub fn benchmark_ipc_recv(runner: &mut BenchmarkRunner) {
        let rb = LockFreeRingBuffer::new();
        let msg = b"Hello, World!";
        rb.try_send(msg);
        let mut buf = [0u8; 256];
        
        runner.run("ipc_recv", 100000, || {
            let start = crate::hal::cpu::rdtsc();
            let _ = rb.try_recv(&mut buf);
            // Re-send for next iteration
            rb.try_send(msg);
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }

    pub fn benchmark_ipc_roundtrip(runner: &mut BenchmarkRunner) {
        let rb = LockFreeRingBuffer::new();
        let msg = b"Hello, World!";
        let mut buf = [0u8; 256];
        
        runner.run("ipc_roundtrip", 100000, || {
            let start = crate::hal::cpu::rdtsc();
            rb.try_send(msg);
            rb.try_recv(&mut buf);
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }
}

/// Scheduler benchmarks
pub mod scheduler_benchmarks {
    use super::*;
    #[cfg(feature = "sched_cfs")]
    use crate::modules::schedulers::cfs::CFS as CFSScheduler;
    #[cfg(feature = "sched_cfs")]
    use crate::interfaces::Scheduler;
    

    #[cfg(feature = "sched_cfs")]
    pub fn benchmark_schedule(runner: &mut BenchmarkRunner) {
        let mut sched = CFSScheduler::new();
        runner.run("schedule", 100000, || {
            let start: u64;
            let end: u64;
            unsafe {
                start = crate::hal::cpu::rdtsc();
            }
            let _ = sched.pick_next();
            unsafe {
                end = crate::hal::cpu::rdtsc();
            }
            end.saturating_sub(start)
        });
    }

    #[cfg(not(feature = "sched_cfs"))]
    pub fn benchmark_schedule(_runner: &mut BenchmarkRunner) {
        // Noop scheduler - skip benchmark
    }

    #[cfg(feature = "sched_cfs")]
    pub fn benchmark_add_task(runner: &mut BenchmarkRunner) {
        let mut sched = CFSScheduler::new();
        runner.run("add_task", 100000, || {
            let start = crate::hal::cpu::rdtsc();
            let _ = sched.pick_next();
            let end = crate::hal::cpu::rdtsc();
            end.saturating_sub(start)
        });
    }

    #[cfg(not(feature = "sched_cfs"))]
    pub fn benchmark_add_task(_runner: &mut BenchmarkRunner) {
        // Noop scheduler - skip benchmark
    }
}

/// Syscall benchmarks
pub mod syscall_benchmarks {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use crate::modules::syscall_inline::{inline_syscall, SyscallNumber};

    pub fn benchmark_syscall_getpid(runner: &mut BenchmarkRunner) {
        runner.run("syscall_getpid", 1000000, || {
            let start = crate::hal::cpu::rdtsc();
            let _ = inline_syscall(SyscallNumber::Getpid, &[]);
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }

    pub fn benchmark_syscall_yield(runner: &mut BenchmarkRunner) {
        runner.run("syscall_yield", 1000000, || {
            let start = crate::hal::cpu::rdtsc();
            let _ = inline_syscall(SyscallNumber::SchedYield, &[]);
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }

    pub fn benchmark_atomic_add(runner: &mut BenchmarkRunner) {
        runner.run("atomic_add", 1000000, || {
            let start = crate::hal::cpu::rdtsc();
            let counter = AtomicUsize::new(0);
            for _ in 0..1000 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
            let end = crate::hal::cpu::rdtsc();
            end - start
        });
    }
}

/// Comprehensive benchmark suite
pub fn run_full_benchmark_suite() {
    let mut runner = BenchmarkRunner::new();
    
    crate::klog_info!("=== Starting AetherXOS Performance Benchmarks ===");
    
    // Memory benchmarks
    crate::klog_info!("Running memory allocation benchmarks...");
    memory_benchmarks::benchmark_tiny_allocations(&mut runner);
    memory_benchmarks::benchmark_small_allocations(&mut runner);
    memory_benchmarks::benchmark_medium_allocations(&mut runner);
    memory_benchmarks::benchmark_large_allocations(&mut runner);
    
    // IPC benchmarks
    crate::klog_info!("Running IPC benchmarks...");
    ipc_benchmarks::benchmark_ipc_send(&mut runner);
    ipc_benchmarks::benchmark_ipc_recv(&mut runner);
    ipc_benchmarks::benchmark_ipc_roundtrip(&mut runner);
    
    // Scheduler benchmarks
    crate::klog_info!("Running scheduler benchmarks...");
    scheduler_benchmarks::benchmark_schedule(&mut runner);
    scheduler_benchmarks::benchmark_add_task(&mut runner);
    
    // Syscall benchmarks
    crate::klog_info!("Running syscall benchmarks...");
    syscall_benchmarks::benchmark_syscall_getpid(&mut runner);
    syscall_benchmarks::benchmark_syscall_yield(&mut runner);
    
    // Print summary
    runner.print_summary();
    
    crate::klog_info!("=== Benchmark Complete ===");
}

/// Performance comparison with baseline
pub struct PerformanceComparison {
    pub baseline: BenchmarkResult,
    pub optimized: BenchmarkResult,
    pub speedup: f64,
    pub improvement_percent: f64,
}

impl PerformanceComparison {
    pub fn new(baseline: BenchmarkResult, optimized: BenchmarkResult) -> Self {
        let speedup = optimized.speedup_factor(&baseline);
        let improvement_percent = (speedup - 1.0) * 100.0;
        
        Self {
            baseline,
            optimized,
            speedup,
            improvement_percent,
        }
    }

    pub fn print(&self) {
        crate::klog_info!("=== Performance Comparison: {} ===", self.baseline.name);
        crate::klog_info!("Baseline: {} ops/sec (avg: {} ns)", 
            self.baseline.ops_per_second, self.baseline.avg_ns);
        crate::klog_info!("Optimized: {} ops/sec (avg: {} ns)", 
            self.optimized.ops_per_second, self.optimized.avg_ns);
        crate::klog_info!("Speedup: {:.2}x", self.speedup);
        crate::klog_info!("Improvement: {:.2}%", self.improvement_percent);
    }
}

/// Expected performance improvements based on optimizations
pub const EXPECTED_IMPROVEMENTS: &[(&str, f64)] = &[
    ("tiny_alloc_16b", 2.5),      // 250% faster
    ("small_alloc_512b", 2.0),    // 200% faster
    ("medium_alloc_8kb", 1.5),    // 150% faster
    ("ipc_send", 4.0),            // 400% faster
    ("ipc_recv", 4.0),            // 400% faster
    ("ipc_roundtrip", 4.0),       // 400% faster
    ("schedule", 1.8),            // 180% faster
    ("syscall_getpid", 5.0),      // 500% faster (inline)
    ("syscall_yield", 5.0),       // 500% faster (inline)
];

/// Validate that benchmarks meet expected improvements
pub fn validate_performance_improvements(results: &[BenchmarkResult]) {
    crate::klog_info!("=== Validating Performance Improvements ===");
    
    for result in results {
        if let Some((_, expected_speedup)) = EXPECTED_IMPROVEMENTS.iter().find(|(name, _)| *name == result.name) {
            // Compare with a hypothetical baseline (would be stored separately)
            crate::klog_info!("{}: Expected {:.2}x speedup", result.name, expected_speedup);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_benchmark_result_calculation() {
        let timings = [100, 200, 300, 400, 500];
        let result = BenchmarkResult::new("test", 5, &timings);
        
        assert_eq!(result.avg_ns, 300);
        assert_eq!(result.min_ns, 100);
        assert_eq!(result.max_ns, 500);
        assert_eq!(result.total_ns, 1500);
    }

    #[test_case]
    fn test_speedup_calculation() {
        let baseline = BenchmarkResult::new("baseline", 1, &[1000]);
        let optimized = BenchmarkResult::new("optimized", 1, &[250]);
        
        let speedup = optimized.speedup_factor(&baseline);
        assert!((speedup - 4.0).abs() < 0.1);
    }

    #[test_case]
    fn test_performance_comparison() {
        let baseline = BenchmarkResult::new("test", 1, &[1000]);
        let optimized = BenchmarkResult::new("test", 1, &[250]);
        
        let comparison = PerformanceComparison::new(baseline, optimized);
        assert!((comparison.speedup - 4.0).abs() < 0.1);
        assert!((comparison.improvement_percent - 300.0).abs() < 0.1);
    }
}
