use crate::modules::posix::time::{clock_gettime, PosixClockId};

#[test_case]
fn deep_time_clock_gettime_works() {
    let mono = clock_gettime(PosixClockId::Monotonic);
    let real = clock_gettime(PosixClockId::Realtime);
    assert!(mono.sec >= 0);
    assert!(real.sec >= 0);
}
