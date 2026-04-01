use alloc::collections::VecDeque;
use spin::Mutex;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxTimeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxInputEvent {
    pub time: LinuxTimeval,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

pub static EVENT_QUEUE: Mutex<Option<VecDeque<LinuxInputEvent>>> = Mutex::new(None);

struct MouseState {
    cycle: u8,
    packet: [u8; 3],
}

static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState {
    cycle: 0,
    packet: [0; 3],
});

pub fn init() {
    *EVENT_QUEUE.lock() = Some(VecDeque::with_capacity(128));

    // Enable PS2 Mouse device
    use x86_64::instructions::port::Port;
    unsafe {
        let mut port64 = Port::<u8>::new(0x64);
        let mut port60 = Port::<u8>::new(0x60);

        // Enable auxiliary mouse device
        port64.write(0xA8);

        // Get Compac status byte
        port64.write(0x20);
        // Wait for data
        while (port64.read() & 1) == 0 {}
        let mut status = port60.read();

        // Enable IRQ12
        status |= 2;
        // Turn off disable mouse clock
        status &= !0x20;

        // Set Compac status byte
        port64.write(0x60);
        port60.write(status);

        // Tell mouse to use default settings
        port64.write(0xD4);
        port60.write(0xF6);
        // wait for ack
        while (port64.read() & 1) == 0 {}
        let _ack = port60.read();

        // Enable data reporting
        port64.write(0xD4);
        port60.write(0xF4);
        // wait for ack
        while (port64.read() & 1) == 0 {}
        let _ack = port60.read();
    }
}

pub fn push_event(type_: u16, code: u16, value: i32) {
    if let Some(ref mut q) = *EVENT_QUEUE.lock() {
        if q.len() >= 128 {
            q.pop_front();
        }
        let tick = crate::kernel::watchdog::global_tick();
        q.push_back(LinuxInputEvent {
            time: LinuxTimeval {
                tv_sec: (tick / 1_000_000_000) as i64,
                tv_usec: ((tick % 1_000_000_000) / 1000) as i64,
            },
            type_,
            code,
            value,
        });
    }
}

pub fn pop_event() -> Option<LinuxInputEvent> {
    if let Some(ref mut q) = *EVENT_QUEUE.lock() {
        q.pop_front()
    } else {
        None
    }
}

pub fn handle_keyboard_irq(_irq: u8) {
    use x86_64::instructions::port::Port;
    let scancode: u8 = unsafe { Port::new(0x60).read() };
    let released = (scancode & 0x80) != 0;
    let keycode = scancode & 0x7F;
    push_event(1, keycode as u16, if released { 0 } else { 1 });
}

pub fn handle_mouse_irq(_irq: u8) {
    use x86_64::instructions::port::Port;
    let data: u8 = unsafe { Port::new(0x60).read() };

    let mut state = MOUSE_STATE.lock();
    let idx = state.cycle as usize;
    state.packet[idx] = data;

    if state.cycle == 0 {
        // Validation: Bit 3 is always 1 in mouse packet byte 1.
        // If not, we are out of sync.
        if (data & 0x08) != 0 {
            state.cycle += 1;
        }
    } else if state.cycle == 1 {
        state.cycle += 1;
    } else if state.cycle == 2 {
        state.cycle = 0;
        let p = state.packet;

        // Byte 0: Y_OVF | X_OVF | Y_SIGN | X_SIGN | 1 | M_BTN | R_BTN | L_BTN
        // If overflow bits are set, the movement data is unreliable — discard this packet.
        let x_overflow = (p[0] & 0b0100_0000) != 0;
        let y_overflow = (p[0] & 0b1000_0000) != 0;
        if x_overflow || y_overflow {
            return; // Drop corrupted packet
        }

        let left_btn = (p[0] & 0b0000_0001) != 0;
        let right_btn = (p[0] & 0b0000_0010) != 0;
        let mid_btn = (p[0] & 0b0000_0100) != 0;

        let mut x_movement = p[1] as i32;
        if (p[0] & 0b0001_0000) != 0 {
            x_movement -= 256;
        }

        let mut y_movement = p[2] as i32;
        if (p[0] & 0b0010_0000) != 0 {
            y_movement -= 256;
        }

        // Linux EV_REL keys: REL_X = 0, REL_Y = 1
        if x_movement != 0 {
            push_event(2, 0, x_movement);
        }

        // PS/2 Y is bottom-to-top, Linux REL_Y is usually top-to-bottom. We negate it.
        if y_movement != 0 {
            push_event(2, 1, -y_movement);
        }

        // BTN_LEFT (0x110 = 272)
        push_event(1, 272, if left_btn { 1 } else { 0 });
        // BTN_RIGHT (0x111 = 273)
        push_event(1, 273, if right_btn { 1 } else { 0 });
        // BTN_MIDDLE (0x112 = 274)
        push_event(1, 274, if mid_btn { 1 } else { 0 });

        // EV_SYN (0), SYN_REPORT (0) indicating complete packet
        push_event(0, 0, 0);
    }
}
