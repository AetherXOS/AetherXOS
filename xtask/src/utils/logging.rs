use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

static VERBOSE: AtomicBool = AtomicBool::new(false);
static QUIET: AtomicBool = AtomicBool::new(false);

pub fn set_verbosity(verbose: bool, quiet: bool) {
    VERBOSE.store(verbose, Ordering::SeqCst);
    QUIET.store(quiet, Ordering::SeqCst);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

pub fn is_quiet() -> bool {
    QUIET.load(Ordering::SeqCst)
}

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
    if is_quiet() {
        return;
    }
    println!(
        " {}{} {:12} {} {}",
        BG_CYAN, FG_BLACK, "ABOUT", RESET, about
    );
    println!(
        " {}{} {:12} {} {}",
        BG_CYAN, FG_BLACK, "SYSTEM", RESET, system
    );
    println!(
        " {}{} {:12} {} {}",
        BG_CYAN, FG_BLACK, "TARGET", RESET, target
    );
    println!();
}

fn get_timestamp() -> String {
    let now = SystemTime::now();
    let datetime: DateTime<Utc> = now.into();
    datetime.format("%H:%M:%S%.3f").to_string()
}

fn use_json_output() -> bool {
    std::env::var("XTASK_LOG_JSON")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || std::env::var("XTASK_LOG_FORMAT")
            .map(|v| v.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
}

pub fn log(level: &str, module: &str, message: &str, kv: &[(&str, &str)]) {
    if is_quiet() && level != "ERROR" && level != "WARN" {
        return;
    }

    let ts = get_timestamp();

    if use_json_output() {
        let mut fields = Map::new();
        for (k, v) in kv {
            fields.insert((*k).to_string(), Value::String((*v).to_string()));
        }
        let record = json!({
            "ts": ts,
            "level": level,
            "module": module,
            "message": message,
            "fields": fields,
        });
        println!("{}", record);
        return;
    }

    let (bg_color, fg_color) = match level {
        "ERROR" => (BG_RED, FG_WHITE),
        "WARN" => (BG_YELLOW, FG_BLACK),
        "EXEC" => (BG_MAGENTA, FG_WHITE),
        "READY" | "SUCCESS" => (BG_GREEN, FG_BLACK),
        _ => (BG_BLUE, FG_WHITE),
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

pub fn warn(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("WARN", module, message, kv);
}

#[allow(dead_code)]
pub fn event(module: &str, event: &str, status: &str, kv: &[(&str, &str)]) {
    let mut fields = Vec::with_capacity(kv.len() + 2);
    fields.push(("event", event));
    fields.push(("status", status));
    fields.extend_from_slice(kv);
    log("INFO", module, "event", &fields);
}

pub fn exec(module: &str, command: &str) {
    if is_verbose() {
        log("EXEC", module, command, &[]);
    }
}

#[allow(dead_code)]
pub fn error(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("ERROR", module, message, kv);
}

pub fn success(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("SUCCESS", module, message, kv);
}

pub trait ReadyDetails {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)>;
}

impl ReadyDetails for &str {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        vec![(Cow::Borrowed("path"), self.to_string())]
    }
}

impl ReadyDetails for String {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        vec![(Cow::Borrowed("path"), self)]
    }
}

impl ReadyDetails for &String {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        vec![(Cow::Borrowed("path"), self.clone())]
    }
}

impl ReadyDetails for Cow<'_, str> {
    fn into_pairs(self) -> Vec<(Cow<'static, str>, String)> {
        vec![(Cow::Borrowed("path"), self.into_owned())]
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
