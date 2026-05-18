use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TerminalCell {
    pub char: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VirtualTerminal {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<TerminalCell>,
}

impl VirtualTerminal {
    pub fn new(width: u16, height: u16) -> Self {
        let cells = vec![
            TerminalCell {
                char: ' ',
                fg: (255, 255, 255),
                bg: (0, 0, 0)
            };
            (width * height) as usize
        ];
        Self {
            width,
            height,
            cells,
        }
    }

    pub fn set_cell(&mut self, x: u16, y: u16, cell: TerminalCell) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.cells[idx] = cell;
        }
    }
}

pub static VTERM: Lazy<Mutex<VirtualTerminal>> =
    Lazy::new(|| Mutex::new(VirtualTerminal::new(100, 30)));

pub fn update_vterm(width: u16, height: u16, cells: Vec<TerminalCell>) {
    if let Ok(mut term) = VTERM.lock() {
        *term = VirtualTerminal {
            width,
            height,
            cells,
        };
    }
}
