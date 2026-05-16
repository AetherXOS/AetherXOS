use std::sync::mpsc;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::fs::OpenOptions;
use std::io::Write;

pub enum LogCommand {
    Write(String),
    Flush,
    Terminate,
}

pub static LOG_TX: Lazy<Mutex<Option<mpsc::Sender<LogCommand>>>> = Lazy::new(|| Mutex::new(None));

pub fn init_background_writer(path: &std::path::Path) -> mpsc::Sender<LogCommand> {
    let (tx, rx) = mpsc::channel();
    let path_clone = path.to_path_buf();

    std::thread::spawn(move || {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path_clone)
            .expect("Failed to open log file");

        while let Ok(cmd) = rx.recv() {
            match cmd {
                LogCommand::Write(msg) => {
                    let _ = writeln!(file, "{}", msg);
                }
                LogCommand::Flush => {
                    let _ = file.flush();
                }
                LogCommand::Terminate => break,
            }
        }
    });

    tx
}
