#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate hypercore;

use hypercore::interfaces::memory::HeapAllocator;
#[cfg(any(
    feature = "sched_round_robin",
    feature = "sched_fifo",
    feature = "sched_lottery"
))]
use hypercore::interfaces::Scheduler;
#[cfg(any(
    feature = "sched_round_robin",
    feature = "sched_fifo",
    feature = "sched_lottery"
))]
use hypercore::interfaces::{KernelTask, TaskId};
use hypercore::modules::allocators::selector::ActiveHeapAllocator;
#[cfg(feature = "sched_lottery")]
use hypercore::modules::schedulers::Lottery;
#[cfg(feature = "sched_round_robin")]
use hypercore::modules::schedulers::RoundRobin;
#[cfg(feature = "sched_fifo")]
use hypercore::modules::schedulers::FIFO;

#[global_allocator]
static ALLOCATOR: ActiveHeapAllocator = ActiveHeapAllocator::new();

const HEAP_START: usize = 0x_5555_5555_0000;
const HEAP_SIZE: usize = 16 * 1024 * 1024;

#[cfg(any(
    feature = "sched_round_robin",
    feature = "sched_fifo",
    feature = "sched_lottery"
))]
fn mk_task(id: usize) -> KernelTask {
    KernelTask::new(TaskId(id), 1, 0, 0, 0x1000 + (id as u64) * 0x1000, 0, 0)
}

#[cfg(feature = "sched_lottery")]
fn mk_task_with_priority(id: usize, priority: u8) -> KernelTask {
    KernelTask::new(
        TaskId(id),
        priority,
        0,
        0,
        0x1000 + (id as u64) * 0x1000,
        0,
        0,
    )
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ALLOCATOR.init(HEAP_START, HEAP_SIZE);
    test_main();
    loop {}
}

#[test_case]
#[cfg(feature = "sched_round_robin")]
fn test_round_robin_basic() {
    let mut rr = RoundRobin::new();
    let t1 = mk_task(1);
    let t2 = mk_task(2);

    rr.add_task(t1.clone());
    rr.add_task(t2.clone());

    assert_eq!(rr.pick_next(), Some(TaskId(1)));
    assert_eq!(rr.pick_next(), Some(TaskId(2)));
    assert_eq!(rr.pick_next(), Some(TaskId(1)));
}

#[test_case]
#[cfg(feature = "sched_fifo")]
fn test_fifo_basic() {
    let mut fifo = FIFO::new();
    let t1 = mk_task(10);
    let t2 = mk_task(20);

    fifo.add_task(t1);
    fifo.add_task(t2);

    assert_eq!(fifo.pick_next(), Some(TaskId(10)));
    assert_eq!(fifo.pick_next(), Some(TaskId(20)));
    assert_eq!(fifo.pick_next(), None);
}

#[test_case]
#[cfg(feature = "sched_lottery")]
fn test_lottery_single_task_always_selected() {
    let mut lottery = Lottery::new();
    lottery.add_task(mk_task(42));

    for _ in 0..16 {
        assert_eq!(lottery.pick_next(), Some(TaskId(42)));
    }
}

#[test_case]
#[cfg(feature = "sched_lottery")]
fn test_lottery_two_tasks_returns_valid_ids() {
    let mut lottery = Lottery::new();
    lottery.add_task(mk_task(7));
    lottery.add_task(mk_task(9));

    for _ in 0..32 {
        let picked = lottery.pick_next();
        assert!(matches!(picked, Some(TaskId(7)) | Some(TaskId(9))));
    }
}

#[test_case]
#[cfg(feature = "sched_lottery")]
fn test_lottery_remove_task_excludes_removed_id() {
    let mut lottery = Lottery::new();
    lottery.add_task(mk_task(100));
    lottery.add_task(mk_task(200));
    lottery.remove_task(TaskId(100));

    for _ in 0..32 {
        assert_eq!(lottery.pick_next(), Some(TaskId(200)));
    }
}

#[test_case]
#[cfg(feature = "sched_lottery")]
fn test_lottery_higher_priority_is_observed_more_often() {
    let mut lottery = Lottery::new();
    lottery.add_task(mk_task_with_priority(1, 1));
    lottery.add_task(mk_task_with_priority(2, 200));

    let mut high = 0usize;
    let mut low = 0usize;
    for _ in 0..256 {
        match lottery.pick_next() {
            Some(TaskId(1)) => high += 1,
            Some(TaskId(2)) => low += 1,
            _ => {}
        }
    }

    assert!(high > low);
}
