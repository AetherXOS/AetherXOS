use super::*;

#[test_case]
fn winsize_roundtrip_between_master_and_slave() {
    let pair = PtyPair::new(7);
    let mut master = PtyMaster::new(7, pair.clone());
    let mut slave = PtySlave::new(7, pair);

    let set = WinSize {
        ws_row: 61,
        ws_col: 144,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let set_ptr = &set as *const WinSize as u64;
    assert_eq!(master.ioctl(TIOCSWINSZ, set_ptr), Ok(0));

    let mut out = WinSize::default();
    let out_ptr = &mut out as *mut WinSize as u64;
    assert_eq!(slave.ioctl(TIOCGWINSZ, out_ptr), Ok(0));
    assert_eq!(out.ws_row, 61);
    assert_eq!(out.ws_col, 144);
}

#[test_case]
fn pgrp_roundtrip_on_slave() {
    let pair = PtyPair::new(8);
    let mut slave = PtySlave::new(8, pair);

    let set: i32 = 1234;
    let set_ptr = &set as *const i32 as u64;
    assert_eq!(slave.ioctl(TIOCSPGRP, set_ptr), Ok(0));

    let mut out: i32 = 0;
    let out_ptr = &mut out as *mut i32 as u64;
    assert_eq!(slave.ioctl(TIOCGPGRP, out_ptr), Ok(0));
    assert_eq!(out, 1234);
}

#[test_case]
fn pgrp_roundtrip_on_master() {
    let pair = PtyPair::new(11);
    let mut master = PtyMaster::new(11, pair);

    let set: i32 = 2468;
    let set_ptr = &set as *const i32 as u64;
    assert_eq!(master.ioctl(TIOCSPGRP, set_ptr), Ok(0));

    let mut out: i32 = 0;
    let out_ptr = &mut out as *mut i32 as u64;
    assert_eq!(master.ioctl(TIOCGPGRP, out_ptr), Ok(0));
    assert_eq!(out, 2468);
}

#[test_case]
fn pgrp_ioctl_rejects_non_positive_values() {
    let pair = PtyPair::new(10);
    let mut slave = PtySlave::new(10, pair);

    let zero: i32 = 0;
    let neg: i32 = -44;
    assert_eq!(slave.ioctl(TIOCSPGRP, &zero as *const i32 as u64), Err("EINVAL"));
    assert_eq!(slave.ioctl(TIOCSPGRP, &neg as *const i32 as u64), Err("EINVAL"));
}

#[test_case]
fn winsize_ioctl_rejects_null_pointer() {
    let pair = PtyPair::new(9);
    let mut master = PtyMaster::new(9, pair.clone());
    let mut slave = PtySlave::new(9, pair);

    assert_eq!(master.ioctl(TIOCSWINSZ, 0), Err("EFAULT"));
    assert_eq!(slave.ioctl(TIOCSWINSZ, 0), Err("EFAULT"));
}

#[test_case]
fn winsize_change_with_foreground_group() {
    let pair = PtyPair::new(12);
    let mut slave = PtySlave::new(12, pair.clone());

    let pgrp: i32 = 5678;
    assert_eq!(slave.ioctl(TIOCSPGRP, &pgrp as *const i32 as u64), Ok(0));

    let new_size = WinSize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = slave.ioctl(TIOCSWINSZ, &new_size as *const WinSize as u64);
    assert_eq!(result, Ok(0), "TIOCSWINSZ should succeed even with foreground group");
}

#[test_case]
fn winsize_change_with_no_foreground_group() {
    let pair = PtyPair::new(13);
    let mut slave = PtySlave::new(13, pair);

    let new_size = WinSize {
        ws_row: 30,
        ws_col: 100,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = slave.ioctl(TIOCSWINSZ, &new_size as *const WinSize as u64);
    assert_eq!(result, Ok(0), "TIOCSWINSZ should succeed with no foreground group");

    let mut out = WinSize::default();
    slave.ioctl(TIOCGWINSZ, &mut out as *mut WinSize as u64).ok();
    assert_eq!(out.ws_row, 30);
    assert_eq!(out.ws_col, 100);
}

#[test_case]
fn master_winsize_change_sends_sigwinch() {
    let pair = PtyPair::new(14);
    let mut master = PtyMaster::new(14, pair.clone());
    let mut slave = PtySlave::new(14, pair);

    let pgrp: i32 = 9999;
    slave.ioctl(TIOCSPGRP, &pgrp as *const i32 as u64).ok();

    let new_size = WinSize {
        ws_row: 25,
        ws_col: 90,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = master.ioctl(TIOCSWINSZ, &new_size as *const WinSize as u64);
    assert_eq!(result, Ok(0), "Master TIOCSWINSZ should succeed");

    let mut out = WinSize::default();
    slave.ioctl(TIOCGWINSZ, &mut out as *mut WinSize as u64).ok();
    assert_eq!(out.ws_row, 25);
    assert_eq!(out.ws_col, 90);
}

#[test_case]
fn runtime_config_sets_default_winsize_for_new_pairs() {
    reset_pty_runtime_config();
    configure_pty_runtime(|config| {
        config.default_winsize = WinSize {
            ws_row: 40,
            ws_col: 120,
            ws_xpixel: 1,
            ws_ypixel: 2,
        };
        config.auto_sigwinch_on_resize = false;
    });

    let pair = PtyPair::new(15);
    let mut master = PtyMaster::new(15, pair.clone());
    let mut out = WinSize::default();
    let out_ptr = &mut out as *mut WinSize as u64;

    assert_eq!(master.ioctl(TIOCGWINSZ, out_ptr), Ok(0));
    assert_eq!(out.ws_row, 40);
    assert_eq!(out.ws_col, 120);
    assert_eq!(out.ws_xpixel, 1);
    assert_eq!(out.ws_ypixel, 2);
    assert!(!pty_runtime_config().auto_sigwinch_on_resize);

    reset_pty_runtime_config();
}

#[test_case]
fn runtime_config_snapshot_reflects_mutations() {
    reset_pty_runtime_config();
    configure_pty_runtime(|config| {
        config.default_locked = false;
        config.allow_control_terminal_attach = false;
        config.allow_control_terminal_detach = false;
    });

    let snapshot = pty_runtime_config();
    assert!(!snapshot.default_locked);
    assert!(!snapshot.allow_control_terminal_attach);
    assert!(!snapshot.allow_control_terminal_detach);

    reset_pty_runtime_config();
}

#[test_case]
fn controlling_session_attach_and_detach_are_stateful() {
    let pair = PtyPair::new(15);
    assert_eq!(pair.controlling_session(), None);
    assert_eq!(pair.attach_controlling_session(100, 200), Ok(()));
    assert_eq!(pair.controlling_session(), Some(100));
    assert_eq!(pair.attach_controlling_session(100, 201), Ok(()));
    assert_eq!(pair.controlling_session(), Some(100));
    assert_eq!(pair.detach_controlling_session(100), Ok(()));
    assert_eq!(pair.controlling_session(), None);
}
