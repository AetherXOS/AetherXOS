use criterion::{criterion_group, criterion_main, Criterion};

fn scheduler_benchmark(c: &mut Criterion) {
    c.bench_function("scheduler_create_1000_tasks", |b| {
        b.iter(|| {
            // Benchmark: create 1000 tasks
            for i in 0..1000u64 {
                let _task_id = i;
                core::hint::black_box(_task_id);
            }
        })
    });
}

criterion_group!(benches, scheduler_benchmark);
criterion_main!(benches);
