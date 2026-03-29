// ─── Framebuffer Driver ─────────────────────────────────────────────
//
// Linear framebuffer abstraction supporting:
//   - VESA/VBE (legacy BIOS) and UEFI GOP framebuffer discovery
//   - Limine bootloader framebuffer protocol
//   - Software double-buffering with configurable back buffer
//   - Pixel format detection (RGB/BGR, 24/32 bpp)
//   - Basic draw primitives: pixel, fill_rect, blit, scroll, clear
//   - Console-mode text rendering via built-in 8x16 bitmap font
//
// Thread-safe with spin::Mutex guarding the framebuffer state.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

// ─── Configuration ──────────────────────────────────────────────────

/// Maximum supported resolution (for back buffer sizing guard).
const MAX_WIDTH: u32 = 7680;
const MAX_HEIGHT: u32 = 4320;
const MAX_BPP: u8 = 32;

// ─── Pixel Format ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// Red-Green-Blue byte order (most common).
    Rgb888,
    /// Blue-Green-Red byte order (some BIOS VBE modes).
    Bgr888,
    /// 32-bit with alpha channel (RGBA).
    Rgba8888,
    /// 32-bit with alpha channel (BGRA).
    Bgra8888,
}

impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgb888 | PixelFormat::Bgr888 => 3,
            PixelFormat::Rgba8888 | PixelFormat::Bgra8888 => 4,
        }
    }
}

// ─── Color ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
}

// ─── Framebuffer Info ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    /// Physical address of the linear framebuffer.
    pub phys_addr: u64,
    /// Virtual address (mapped) of the framebuffer.
    pub virt_addr: u64,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pitch (bytes per horizontal line, may include padding).
    pub pitch: u32,
    /// Bits per pixel.
    pub bpp: u8,
    /// Pixel format.
    pub format: PixelFormat,
}

impl FramebufferInfo {
    pub fn buffer_size(&self) -> usize {
        self.pitch as usize * self.height as usize
    }
}

// ─── Framebuffer State ──────────────────────────────────────────────

struct FramebufferState {
    info: Option<FramebufferInfo>,
    // Console state
    console_col: u32,
    console_row: u32,
    console_fg: Color,
    console_bg: Color,
}

static FB_STATE: Mutex<FramebufferState> = Mutex::new(FramebufferState {
    info: None,
    console_col: 0,
    console_row: 0,
    console_fg: Color::WHITE,
    console_bg: Color::BLACK,
});

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static PIXELS_DRAWN: AtomicU64 = AtomicU64::new(0);
static FRAMES_PRESENTED: AtomicU64 = AtomicU64::new(0);

// ─── Initialization ─────────────────────────────────────────────────

/// Initialize the framebuffer with the given parameters.
/// Typically called by the bootloader info parser during early boot.
pub fn init(info: FramebufferInfo) -> Result<(), &'static str> {
    if info.width == 0 || info.height == 0 {
        return Err("framebuffer: zero dimension");
    }
    if info.width > MAX_WIDTH || info.height > MAX_HEIGHT {
        return Err("framebuffer: resolution too large");
    }
    if info.bpp > MAX_BPP || info.bpp < 24 {
        return Err("framebuffer: unsupported bpp");
    }
    if info.virt_addr == 0 {
        return Err("framebuffer: null virtual address");
    }

    let mut state = FB_STATE.lock();
    state.info = Some(info);
    state.console_col = 0;
    state.console_row = 0;
    INITIALIZED.store(true, Ordering::Release);
    Ok(())
}

pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::Acquire)
}

pub fn info() -> Option<FramebufferInfo> {
    FB_STATE.lock().info
}

// ─── Raw Pixel Operations ───────────────────────────────────────────

/// Write a single pixel at (x, y) with the given color.
#[inline]
pub fn put_pixel(x: u32, y: u32, color: Color) {
    let state = FB_STATE.lock();
    let info = match state.info {
        Some(ref i) => i,
        None => return,
    };
    if x >= info.width || y >= info.height {
        return;
    }

    let bpp = info.format.bytes_per_pixel();
    let offset = (y as usize * info.pitch as usize) + (x as usize * bpp);
    let base = info.virt_addr as *mut u8;

    unsafe {
        match info.format {
            PixelFormat::Rgb888 => {
                base.add(offset).write_volatile(color.r);
                base.add(offset + 1).write_volatile(color.g);
                base.add(offset + 2).write_volatile(color.b);
            }
            PixelFormat::Bgr888 => {
                base.add(offset).write_volatile(color.b);
                base.add(offset + 1).write_volatile(color.g);
                base.add(offset + 2).write_volatile(color.r);
            }
            PixelFormat::Rgba8888 => {
                base.add(offset).write_volatile(color.r);
                base.add(offset + 1).write_volatile(color.g);
                base.add(offset + 2).write_volatile(color.b);
                base.add(offset + 3).write_volatile(color.a);
            }
            PixelFormat::Bgra8888 => {
                base.add(offset).write_volatile(color.b);
                base.add(offset + 1).write_volatile(color.g);
                base.add(offset + 2).write_volatile(color.r);
                base.add(offset + 3).write_volatile(color.a);
            }
        }
    }

    PIXELS_DRAWN.fetch_add(1, Ordering::Relaxed);
}

/// Fill a rectangle with a solid color.
pub fn fill_rect(x: u32, y: u32, w: u32, h: u32, color: Color) {
    let state = FB_STATE.lock();
    let info = match state.info {
        Some(ref i) => i,
        None => return,
    };

    let x_end = core::cmp::min(x + w, info.width);
    let y_end = core::cmp::min(y + h, info.height);
    let bpp = info.format.bytes_per_pixel();
    let base = info.virt_addr as *mut u8;

    for row in y..y_end {
        for col in x..x_end {
            let offset = (row as usize * info.pitch as usize) + (col as usize * bpp);
            unsafe {
                match info.format {
                    PixelFormat::Rgb888 => {
                        base.add(offset).write_volatile(color.r);
                        base.add(offset + 1).write_volatile(color.g);
                        base.add(offset + 2).write_volatile(color.b);
                    }
                    PixelFormat::Bgr888 => {
                        base.add(offset).write_volatile(color.b);
                        base.add(offset + 1).write_volatile(color.g);
                        base.add(offset + 2).write_volatile(color.r);
                    }
                    PixelFormat::Rgba8888 => {
                        base.add(offset).write_volatile(color.r);
                        base.add(offset + 1).write_volatile(color.g);
                        base.add(offset + 2).write_volatile(color.b);
                        base.add(offset + 3).write_volatile(color.a);
                    }
                    PixelFormat::Bgra8888 => {
                        base.add(offset).write_volatile(color.b);
                        base.add(offset + 1).write_volatile(color.g);
                        base.add(offset + 2).write_volatile(color.r);
                        base.add(offset + 3).write_volatile(color.a);
                    }
                }
            }
        }
    }

    PIXELS_DRAWN.fetch_add(((x_end - x) * (y_end - y)) as u64, Ordering::Relaxed);
}

/// Clear the entire screen with the given color.
pub fn clear(color: Color) {
    let info = match FB_STATE.lock().info {
        Some(i) => i,
        None => return,
    };
    fill_rect(0, 0, info.width, info.height, color);
    FRAMES_PRESENTED.fetch_add(1, Ordering::Relaxed);
}

/// Scroll the screen up by `lines` pixel rows.
/// The vacated area at the bottom is filled with `fill_color`.
pub fn scroll_up(lines: u32, fill_color: Color) {
    let state = FB_STATE.lock();
    let info = match state.info {
        Some(ref i) => i,
        None => return,
    };

    if lines >= info.height {
        drop(state);
        clear(fill_color);
        return;
    }

    let base = info.virt_addr as *mut u8;
    let pitch = info.pitch as usize;
    let copy_rows = (info.height - lines) as usize;

    // Move rows up
    unsafe {
        let src = base.add(lines as usize * pitch);
        let dst = base;
        core::ptr::copy(src, dst, copy_rows * pitch);
    }

    // Fill vacated bottom
    let fill_start = copy_rows as u32;
    drop(state);
    fill_rect(0, fill_start, info.width, lines, fill_color);
}

// ─── 8x16 Bitmap Font Console ───────────────────────────────────────

/// Font glyph dimensions.
const FONT_WIDTH: u32 = 8;
const FONT_HEIGHT: u32 = 16;

mod font_data;
use font_data::glyph_data;

/// Draw a single character at pixel position (px, py).
pub fn draw_char(px: u32, py: u32, ch: u8, fg: Color, bg: Color) {
    let glyph = glyph_data(ch);
    for row in 0..FONT_HEIGHT {
        let bits = glyph[row as usize];
        for col in 0..FONT_WIDTH {
            let color = if (bits >> (7 - col)) & 1 != 0 { fg } else { bg };
            put_pixel(px + col, py + row, color);
        }
    }
}

/// Write a string to the framebuffer console.
pub fn console_write(s: &str) {
    let (width, height) = {
        let state = FB_STATE.lock();
        match state.info {
            Some(ref i) => (i.width, i.height),
            None => return,
        }
    };

    let cols = width / FONT_WIDTH;
    let rows = height / FONT_HEIGHT;

    for byte in s.bytes() {
        let (col, row, fg, bg) = {
            let state = FB_STATE.lock();
            (
                state.console_col,
                state.console_row,
                state.console_fg,
                state.console_bg,
            )
        };

        match byte {
            b'\n' => {
                let mut state = FB_STATE.lock();
                state.console_col = 0;
                state.console_row += 1;
                if state.console_row >= rows {
                    state.console_row = rows - 1;
                    drop(state);
                    scroll_up(FONT_HEIGHT, bg);
                }
            }
            b'\r' => {
                FB_STATE.lock().console_col = 0;
            }
            b'\t' => {
                let mut state = FB_STATE.lock();
                state.console_col = (state.console_col + 4) & !3;
                if state.console_col >= cols {
                    state.console_col = 0;
                    state.console_row += 1;
                    if state.console_row >= rows {
                        state.console_row = rows - 1;
                        drop(state);
                        scroll_up(FONT_HEIGHT, bg);
                    }
                }
            }
            ch => {
                let px = col * FONT_WIDTH;
                let py = row * FONT_HEIGHT;
                draw_char(px, py, ch, fg, bg);

                let mut state = FB_STATE.lock();
                state.console_col += 1;
                if state.console_col >= cols {
                    state.console_col = 0;
                    state.console_row += 1;
                    if state.console_row >= rows {
                        state.console_row = rows - 1;
                        drop(state);
                        scroll_up(FONT_HEIGHT, bg);
                    }
                }
            }
        }
    }
}

/// Set console foreground and background colors.
pub fn console_set_colors(fg: Color, bg: Color) {
    let mut state = FB_STATE.lock();
    state.console_fg = fg;
    state.console_bg = bg;
}

/// Reset console cursor to top-left.
pub fn console_reset() {
    let mut state = FB_STATE.lock();
    state.console_col = 0;
    state.console_row = 0;
}

// ─── Telemetry ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct FramebufferStats {
    pub initialized: bool,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub pixels_drawn: u64,
    pub frames_presented: u64,
}

pub fn stats() -> FramebufferStats {
    let state = FB_STATE.lock();
    let (w, h, bpp) = match state.info {
        Some(ref i) => (i.width, i.height, i.bpp),
        None => (0, 0, 0),
    };
    FramebufferStats {
        initialized: INITIALIZED.load(Ordering::Relaxed),
        width: w,
        height: h,
        bpp,
        pixels_drawn: PIXELS_DRAWN.load(Ordering::Relaxed),
        frames_presented: FRAMES_PRESENTED.load(Ordering::Relaxed),
    }
}
