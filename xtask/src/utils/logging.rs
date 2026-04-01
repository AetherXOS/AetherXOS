use std::time::SystemTime;
use chrono::{DateTime, Utc};

pub const RESET: &str = "\x1b[0m";
#[allow(dead_code)]
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

pub const FG_BLACK: &str = "\x1b[30m";
pub const FG_WHITE: &str = "\x1b[37m";

pub const BG_BLUE: &str = "\x1b[44m";
pub const BG_CYAN: &str = "\x1b[46m";
pub const BG_GREEN: &str = "\x1b[42m";
pub const BG_YELLOW: &str = "\x1b[43m";
pub const BG_RED: &str = "\x1b[41m";
pub const BG_MAGENTA: &str = "\x1b[45m";
pub const BG_BRIGHT_BLACK: &str = "\x1b[100m";

pub fn print_header(about: &str, system: &str, target: &str) {
    println!(" {}{} {:12} {} {}", BG_CYAN, FG_BLACK, "ABOUT", RESET, about);
    println!(" {}{} {:12} {} {}", BG_CYAN, FG_BLACK, "SYSTEM", RESET, system);
    println!(" {}{} {:12} {} {}", BG_CYAN, FG_BLACK, "TARGET", RESET, target);
    println!();
}

fn get_timestamp() -> String {
    let now = SystemTime::now();
    let datetime: DateTime<Utc> = now.into();
    datetime.format("%H:%M:%S%.3f").to_string()
}

pub fn log(level: &str, module: &str, message: &str, kv: &[(&str, &str)]) {
    let ts = get_timestamp();
    let (bg_color, fg_color) = match level {
        "ERROR" => (BG_RED, FG_WHITE),
        "WARN"  => (BG_YELLOW, FG_BLACK),
        "EXEC"  => (BG_MAGENTA, FG_WHITE),
        "READY" => (BG_GREEN, FG_BLACK),
        _       => (BG_BLUE, FG_WHITE),
    };

    print!("{}[{}]{} ", DIM, ts, RESET);
    print!(" {}{} {:7} {} ", bg_color, fg_color, level, RESET);
    print!("{}{} {:8} {} ", BG_BRIGHT_BLACK, FG_WHITE, module, RESET);
    print!("{} ", message);

    for (k, v) in kv {
        print!("{}{}={}{} ", DIM, k, RESET, v);
    }
    println!();
}

pub fn info(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("INFO", module, message, kv);
}

pub fn exec(module: &str, command: &str) {
    log("EXEC", module, command, &[]);
}

#[allow(dead_code)]
pub fn error(module: &str, message: &str) {
    log("ERROR", module, message, &[]);
}

pub fn ready(module: &str, message: &str, path: &str) {
    log("READY", module, message, &[("path", path)]);
}
