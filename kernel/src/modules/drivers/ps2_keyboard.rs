// ─── PS/2 Keyboard Driver ───────────────────────────────────────────
//
// Fully configurable PS/2 keyboard driver with scancode set 1/2 support,
// key event ring buffer, modifier tracking, and LED control.
//
// Designed for x86_64 (IO port based) with compile-time gating for other archs.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

mod keymap;

pub use keymap::{keycode_to_ascii, Keycode};
use keymap::scancode_set1_to_keycode;

// ─── Configuration ──────────────────────────────────────────────────

/// Maximum queued key events before dropping oldest.
const KEY_BUFFER_SIZE: usize = 256;

/// PS/2 controller IO ports (x86_64).
const PS2_DATA_PORT: u16 = 0x60;
const PS2_STATUS_PORT: u16 = 0x64;
const PS2_COMMAND_PORT: u16 = 0x64;

/// PS/2 controller commands.
const PS2_CMD_READ_CONFIG: u8 = 0x20;
const PS2_CMD_WRITE_CONFIG: u8 = 0x60;
const PS2_CMD_DISABLE_PORT2: u8 = 0xA7;
const PS2_CMD_ENABLE_PORT1: u8 = 0xAE;
const PS2_CMD_SELF_TEST: u8 = 0xAA;
const PS2_CMD_PORT1_TEST: u8 = 0xAB;

/// Keyboard commands (sent to data port).
const KB_CMD_SET_LEDS: u8 = 0xED;
const KB_CMD_ENABLE_SCANNING: u8 = 0xF4;

// ─── Key Event Types ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub scancode: u8,
    pub keycode: Keycode,
    pub state: KeyState,
    pub modifiers: ModifierState,
    pub timestamp: u64,
}

/// Modifier key state bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifierState {
    pub left_shift: bool,
    pub right_shift: bool,
    pub left_ctrl: bool,
    pub right_ctrl: bool,
    pub left_alt: bool,
    pub right_alt: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
    pub scroll_lock: bool,
}

impl ModifierState {
    pub fn shift(&self) -> bool {
        self.left_shift || self.right_shift
    }
    pub fn ctrl(&self) -> bool {
        self.left_ctrl || self.right_ctrl
    }
    pub fn alt(&self) -> bool {
        self.left_alt || self.right_alt
    }
}

/// LED state for keyboard indicators.
#[derive(Debug, Clone, Copy, Default)]
pub struct LedState {
    pub scroll_lock: bool,
    pub num_lock: bool,
    pub caps_lock: bool,
}

impl LedState {
    fn to_byte(self) -> u8 {
        let mut b = 0u8;
        if self.scroll_lock {
            b |= 1;
        }
        if self.num_lock {
            b |= 2;
        }
        if self.caps_lock {
            b |= 4;
        }
        b
    }
}

// ─── Ring Buffer ────────────────────────────────────────────────────

struct KeyRingBuffer {
    buf: [Option<KeyEvent>; KEY_BUFFER_SIZE],
    head: usize,
    tail: usize,
    count: usize,
}

impl KeyRingBuffer {
    const fn new() -> Self {
        Self {
            buf: [None; KEY_BUFFER_SIZE],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, event: KeyEvent) {
        self.buf[self.head] = Some(event);
        self.head = (self.head + 1) % KEY_BUFFER_SIZE;
        if self.count == KEY_BUFFER_SIZE {
            // Overwrite oldest
            self.tail = (self.tail + 1) % KEY_BUFFER_SIZE;
            EVENTS_DROPPED.fetch_add(1, Ordering::Relaxed);
        } else {
            self.count += 1;
        }
    }

    fn pop(&mut self) -> Option<KeyEvent> {
        if self.count == 0 {
            return None;
        }
        let event = self.buf[self.tail].take();
        self.tail = (self.tail + 1) % KEY_BUFFER_SIZE;
        self.count -= 1;
        event
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn len(&self) -> usize {
        self.count
    }
}

// ─── Global State ───────────────────────────────────────────────────

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static IRQ_COUNT: AtomicU64 = AtomicU64::new(0);
static EVENTS_PROCESSED: AtomicU64 = AtomicU64::new(0);
static EVENTS_DROPPED: AtomicU64 = AtomicU64::new(0);
static KEY_BUFFER: Mutex<KeyRingBuffer> = Mutex::new(KeyRingBuffer::new());
static MODIFIER_STATE: Mutex<ModifierState> = Mutex::new(ModifierState {
    left_shift: false,
    right_shift: false,
    left_ctrl: false,
    right_ctrl: false,
    left_alt: false,
    right_alt: false,
    caps_lock: false,
    num_lock: false,
    scroll_lock: false,
});
static EXTENDED_KEY: AtomicBool = AtomicBool::new(false);

// ─── IO Port Helpers ────────────────────────────────────────────────

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack));
    }
    val
}

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn outb(port: u16, val: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("al") val, in("dx") port, options(nomem, nostack));
    }
}

#[cfg(target_arch = "x86_64")]
fn wait_for_input_buffer() {
    for _ in 0..10_000 {
        unsafe {
            if (inb(PS2_STATUS_PORT) & 0x02) == 0 {
                return;
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn wait_for_output_buffer() -> bool {
    for _ in 0..10_000 {
        unsafe {
            if (inb(PS2_STATUS_PORT) & 0x01) != 0 {
                return true;
            }
        }
    }
    false
}

// ─── Driver Interface ───────────────────────────────────────────────

/// Initialize the PS/2 keyboard controller.
pub fn init() -> Result<(), &'static str> {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            // Disable port 2 (mouse) to avoid interference
            outb(PS2_COMMAND_PORT, PS2_CMD_DISABLE_PORT2);
            wait_for_input_buffer();

            // Self-test
            outb(PS2_COMMAND_PORT, PS2_CMD_SELF_TEST);
            if !wait_for_output_buffer() {
                return Err("PS/2 self-test timeout");
            }
            let result = inb(PS2_DATA_PORT);
            if result != 0x55 {
                return Err("PS/2 self-test failed");
            }

            // Test port 1
            outb(PS2_COMMAND_PORT, PS2_CMD_PORT1_TEST);
            if !wait_for_output_buffer() {
                return Err("PS/2 port 1 test timeout");
            }
            let port_test = inb(PS2_DATA_PORT);
            if port_test != 0x00 {
                return Err("PS/2 port 1 test failed");
            }

            // Enable port 1
            outb(PS2_COMMAND_PORT, PS2_CMD_ENABLE_PORT1);
            wait_for_input_buffer();

            // Read config byte and enable IRQ1
            outb(PS2_COMMAND_PORT, PS2_CMD_READ_CONFIG);
            if !wait_for_output_buffer() {
                return Err("PS/2 read config timeout");
            }
            let mut config = inb(PS2_DATA_PORT);
            config |= 0x01; // Enable IRQ1 (keyboard)
            config &= !0x10; // Enable clock for port 1

            outb(PS2_COMMAND_PORT, PS2_CMD_WRITE_CONFIG);
            wait_for_input_buffer();
            outb(PS2_DATA_PORT, config);
            wait_for_input_buffer();

            // Enable scanning
            outb(PS2_DATA_PORT, KB_CMD_ENABLE_SCANNING);
            wait_for_input_buffer();
        }

        INITIALIZED.store(true, Ordering::Release);
        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Err("PS/2 keyboard not supported on this architecture")
    }
}

/// Handle a keyboard IRQ (IRQ1). Called from the interrupt handler.
pub fn handle_irq() {
    IRQ_COUNT.fetch_add(1, Ordering::Relaxed);

    #[cfg(target_arch = "x86_64")]
    {
        let scancode = unsafe { inb(PS2_DATA_PORT) };
        process_scancode(scancode);
    }
}

fn process_scancode(scancode: u8) {
    // Extended key prefix
    if scancode == 0xE0 {
        EXTENDED_KEY.store(true, Ordering::Relaxed);
        return;
    }

    let is_extended = EXTENDED_KEY.swap(false, Ordering::Relaxed);
    let is_release = (scancode & 0x80) != 0;
    let raw = scancode & 0x7F;

    let state = if is_release {
        KeyState::Released
    } else {
        KeyState::Pressed
    };

    // Handle extended keys
    let keycode = if is_extended {
        match raw {
            0x1D => Keycode::RightCtrl,
            0x38 => Keycode::RightAlt,
            0x48 => Keycode::Up,
            0x50 => Keycode::Down,
            0x4B => Keycode::Left,
            0x4D => Keycode::Right,
            0x47 => Keycode::Home,
            0x4F => Keycode::End,
            0x49 => Keycode::PageUp,
            0x51 => Keycode::PageDown,
            0x52 => Keycode::Insert,
            0x53 => Keycode::Delete,
            0x35 => Keycode::NumpadSlash,
            0x1C => Keycode::NumpadEnter,
            _ => Keycode::Unknown,
        }
    } else {
        scancode_set1_to_keycode(raw)
    };

    // Update modifier state
    {
        let mut mods = MODIFIER_STATE.lock();
        match keycode {
            Keycode::LeftShift => mods.left_shift = state == KeyState::Pressed,
            Keycode::RightShift => mods.right_shift = state == KeyState::Pressed,
            Keycode::LeftCtrl => mods.left_ctrl = state == KeyState::Pressed,
            Keycode::RightCtrl => mods.right_ctrl = state == KeyState::Pressed,
            Keycode::LeftAlt => mods.left_alt = state == KeyState::Pressed,
            Keycode::RightAlt => mods.right_alt = state == KeyState::Pressed,
            Keycode::CapsLock if state == KeyState::Pressed => mods.caps_lock = !mods.caps_lock,
            Keycode::NumLock if state == KeyState::Pressed => mods.num_lock = !mods.num_lock,
            Keycode::ScrollLock if state == KeyState::Pressed => {
                mods.scroll_lock = !mods.scroll_lock
            }
            _ => {}
        }
    }

    let modifiers = *MODIFIER_STATE.lock();
    let event = KeyEvent {
        scancode: raw,
        keycode,
        state,
        modifiers,
        timestamp: IRQ_COUNT.load(Ordering::Relaxed),
    };

    KEY_BUFFER.lock().push(event);
    EVENTS_PROCESSED.fetch_add(1, Ordering::Relaxed);
}

/// Read the next key event from the buffer. Returns `None` if empty.
pub fn read_event() -> Option<KeyEvent> {
    KEY_BUFFER.lock().pop()
}

/// Check if there are pending key events.
pub fn has_events() -> bool {
    !KEY_BUFFER.lock().is_empty()
}

/// Number of pending key events.
pub fn pending_count() -> usize {
    KEY_BUFFER.lock().len()
}

/// Set keyboard LEDs (Scroll Lock, Num Lock, Caps Lock).
pub fn set_leds(leds: LedState) {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            wait_for_input_buffer();
            outb(PS2_DATA_PORT, KB_CMD_SET_LEDS);
            wait_for_input_buffer();
            outb(PS2_DATA_PORT, leds.to_byte());
            wait_for_input_buffer();
        }
    }
    let _ = leds;
}

/// Driver telemetry.
#[derive(Debug, Clone, Copy)]
pub struct Ps2KeyboardStats {
    pub initialized: bool,
    pub irq_count: u64,
    pub events_processed: u64,
    pub events_dropped: u64,
    pub pending_events: usize,
}

pub fn stats() -> Ps2KeyboardStats {
    Ps2KeyboardStats {
        initialized: INITIALIZED.load(Ordering::Relaxed),
        irq_count: IRQ_COUNT.load(Ordering::Relaxed),
        events_processed: EVENTS_PROCESSED.load(Ordering::Relaxed),
        events_dropped: EVENTS_DROPPED.load(Ordering::Relaxed),
        pending_events: KEY_BUFFER.lock().len(),
    }
}
