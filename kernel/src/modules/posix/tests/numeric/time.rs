use super::*;

#[test_case]
#[cfg(feature = "posix_time")]
fn time_apis_are_consistent() {
    let mono = clock_gettime(PosixClockId::Monotonic);
    let real = clock_gettime(PosixClockId::Realtime);
    assert!(mono.sec >= 0);
    assert!(real.sec >= 0);

    assert_eq!(PosixClockId::Realtime.as_raw(), crate::modules::posix_consts::time::CLOCK_REALTIME);
    assert_eq!(PosixClockId::Monotonic.as_raw(), crate::modules::posix_consts::time::CLOCK_MONOTONIC);
    assert_eq!(
        PosixClockId::from_raw(crate::modules::posix_consts::time::CLOCK_REALTIME),
        Some(PosixClockId::Realtime)
    );
    assert_eq!(PosixClockId::from_raw(-123), None);
    assert_eq!(
        clock_gettime_raw(crate::modules::posix_consts::time::CLOCK_MONOTONIC)
            .expect("clock_gettime_raw")
            .sec
            >= 0,
        true
    );
    assert!(clock_gettime64(PosixClockId::Monotonic).sec >= 0);

    let res = clock_getres(PosixClockId::Monotonic);
    assert_eq!(res.sec, 0);
    assert!(res.nsec > 0);
    assert!(res.nsec <= 1_000_000_000);
    assert_eq!(
        clock_getres_raw(crate::modules::posix_consts::time::CLOCK_REALTIME)
            .expect("clock_getres_raw")
            .sec,
        0
    );
    assert_eq!(clock_getres_raw(99_999), Err(PosixErrno::Invalid));

    let now_rt = clock_gettime(PosixClockId::Realtime);
    clock_settime(PosixClockId::Realtime, now_rt).expect("clock_settime realtime");
    assert_eq!(clock_settime(PosixClockId::Monotonic, now_rt), Err(PosixErrno::Invalid));
    clock_settime_raw(crate::modules::posix_consts::time::CLOCK_REALTIME, now_rt)
        .expect("clock_settime_raw realtime");
    settimeofday(gettimeofday()).expect("settimeofday");

    let tv = gettimeofday();
    assert!(tv.sec >= 0);
    assert!(tv.usec >= 0);
    assert!(tv.usec < 1_000_000);
    assert!(
        timespec_get(crate::modules::posix_consts::time::TIME_UTC)
            .expect("timespec_get")
            .sec
            >= 0
    );
    assert_eq!(
        timespec_getres(crate::modules::posix_consts::time::TIME_UTC)
            .expect("timespec_getres")
            .sec,
        0
    );
    assert_eq!(timespec_get(0), Err(PosixErrno::Invalid));

    let t = time_now();
    assert!(t >= 0);

    nanosleep(PosixTimespec { sec: 0, nsec: 0 }).expect("nanosleep");
    assert_eq!(
        nanosleep_with_rem(PosixTimespec { sec: 0, nsec: 0 }).expect("nanosleep rem"),
        PosixTimespec { sec: 0, nsec: 0 }
    );
    clock_nanosleep(PosixClockId::Monotonic, 0, PosixTimespec { sec: 0, nsec: 0 })
        .expect("clock_nanosleep rel");
    clock_nanosleep_raw(
        crate::modules::posix_consts::time::CLOCK_MONOTONIC,
        crate::modules::posix_consts::time::TIMER_ABSTIME,
        mono,
    )
    .expect("clock_nanosleep abstime");
    assert_eq!(
        clock_nanosleep_raw(777, 0, PosixTimespec { sec: 0, nsec: 0 }),
        Err(PosixErrno::Invalid)
    );
    usleep(0).expect("usleep");
    sleep(0).expect("sleep");
}
