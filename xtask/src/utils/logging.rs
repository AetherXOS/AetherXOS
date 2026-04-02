use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::time::SystemTime;

pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";

pub const FG_BLACK: &str = "\x1b[30m";
pub const FG_WHITE: &str = "\x1b[37m";
pub const FG_CYAN: &str = "\x1b[36m";
pub const FG_GREEN: &str = "\x1b[32m";
pub const FG_YELLOW: &str = "\x1b[33m";
pub const FG_RED: &str = "\x1b[31m";

pub const BG_BLUE: &str = "\x1b[44m";
pub const BG_CYAN: &str = "\x1b[46m";
pub const BG_GREEN: &str = "\x1b[42m";
pub const BG_YELLOW: &str = "\x1b[43m";
pub const BG_RED: &str = "\x1b[41m";
pub const BG_MAGENTA: &str = "\x1b[45m";
pub const BG_BRIGHT_BLACK: &str = "\x1b[100m";

pub fn print_header(about: &str, system: &str, target: &str) {
    println!("{}+{:-<86}+{}", DIM, "", RESET);
    println!(
        " {}{} XTASK {} {}",
        BG_CYAN, FG_BLACK, RESET, about
    );
    println!(
        " {}{} HOST  {} {}",
        BG_BRIGHT_BLACK, FG_WHITE, RESET, system
    );
    println!(
        " {}{} TARGET {} {}",
        BG_BRIGHT_BLACK, FG_WHITE, RESET, target
    );
    println!("{}+{:-<86}+{}", DIM, "", RESET);
    println!();
}

fn get_timestamp() -> String {
    let now = SystemTime::now();
    let datetime: DateTime<Utc> = now.into();
    datetime.format("%H:%M:%S%.3f").to_string()
}

pub fn log(level: &str, module: &str, message: &str, kv: &[(&str, &str)]) {
    let ts = get_timestamp();
    let (badge_bg, badge_fg, accent_fg) = match level {
        "ERROR" => (BG_RED, FG_WHITE, FG_RED),
        "WARN" => (BG_YELLOW, FG_BLACK, FG_YELLOW),
        "EXEC" => (BG_MAGENTA, FG_WHITE, FG_CYAN),
        "READY" => (BG_GREEN, FG_BLACK, FG_GREEN),
        _ => (BG_BLUE, FG_WHITE, FG_CYAN),
    };

    print!("{}[{}]{} ", DIM, ts, RESET);
    print!(" {}{} {:7} {} ", badge_bg, badge_fg, level, RESET);
    print!("{}{}{} ", accent_fg, module, RESET);
    print!("{}> {}{}", DIM, RESET, message);

    for (k, v) in kv {
        print!("  {}{}{}={}{}{}", DIM, k, RESET, accent_fg, v, RESET);
    }
    println!();
}

pub fn info(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("INFO", module, message, kv);
}

pub fn warn(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("WARN", module, message, kv);
}

pub fn exec(module: &str, command: &str) {
    log("EXEC", module, command, &[]);
}

pub fn error(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("ERROR", module, message, kv);
}

pub trait ReadyDetails {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)>;
}

impl ReadyDetails for &str {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        vec![(Cow::Borrowed("path"), self.to_string())]
    }
}

impl ReadyDetails for &String {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        vec![(Cow::Borrowed("path"), self.clone())]
    }
}

impl ReadyDetails for &Cow<'_, str> {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        vec![(Cow::Borrowed("path"), self.to_string())]
    }
}

impl<const N: usize> ReadyDetails for &[(&str, &str); N] {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        self.iter()
            .map(|(key, value)| (Cow::Owned((*key).to_string()), (*value).to_string()))
            .collect()
    }
}

impl<const N: usize> ReadyDetails for &[(&str, &String); N] {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        self.iter()
            .map(|(key, value)| (Cow::Owned((*key).to_string()), (*value).clone()))
            .collect()
    }
}

pub fn ready<D>(module: &str, message: &str, details: D)
where
    D: ReadyDetails,
{
    let pairs = details.into_pairs();
    let kv: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(key, value)| (key.as_ref(), value.as_str()))
        .collect();
    log("READY", module, message, &kv);
}
