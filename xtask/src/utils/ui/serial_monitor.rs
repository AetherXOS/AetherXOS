use std::{fs::File, io::Read, time::{Duration, Instant}};
use anyhow::{Result, Context};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph, List, ListItem},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Modifier},
    Terminal,
};

pub fn run_serial_monitor() -> Result<()> {
    let qemu_bin = crate::utils::sys::process::Discovery::qemu_system_x86_64()
        .context("qemu-system-x86_64 not found in PATH")?;

    let image_path = match crate::utils::core::paths::WorkspacePaths::find_boot_image() {
        Some(p) => p,
        None => match crate::utils::core::paths::WorkspacePaths::find_kernel_elf() {
            Some(p) => p,
            None => anyhow::bail!("No bootable ISO, image, or kernel binary found. Please build the project first."),
        }
    };

    // Create artifacts directory if not exists
    let log_path = std::path::Path::new("artifacts/qemu_serial.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Clear old log file
    let _ = std::fs::remove_file(log_path);
    let _ = File::create(log_path);

    // Spawn QEMU in background redirecting serial to file and starting GDB stub on port 1234
    let drive_arg = format!("file={},format=raw", image_path.display());
    let mut qemu_child = std::process::Command::new(&qemu_bin)
        .args(&[
            "-m", "1024",
            "-drive", &drive_arg,
            "-serial", "file:artifacts/qemu_serial.log",
            "-s", // Enable GDB server on port 1234
            "-display", "none", // Headless execution to keep everything in the terminal
        ])
        .spawn()
        .context("Failed to spawn QEMU")?;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut logs = Vec::new();
    let mut log_file = File::open(log_path)?;
    let mut log_buffer = String::new();
    let start_time = Instant::now();
    let mut show_gdb_helper = false;

    loop {
        // Read new logs from log file
        let mut temp_buf = Vec::new();
        if let Ok(bytes_read) = log_file.read_to_end(&mut temp_buf) {
            if bytes_read > 0 {
                let s = String::from_utf8_lossy(&temp_buf);
                log_buffer.push_str(&s);
                while let Some(pos) = log_buffer.find('\n') {
                    let line = log_buffer[..pos].trim().to_string();
                    log_buffer = log_buffer[pos + 1..].to_string();
                    if !line.is_empty() {
                        if logs.len() > 30 {
                            logs.remove(0);
                        }
                        logs.push(line);
                    }
                }
            }
        }

        // Check if QEMU child is still alive
        let qemu_status = match qemu_child.try_wait() {
            Ok(Some(status)) => format!("Stopped (Exit Code: {})", status),
            _ => "Running (GDB Server on :1234)".to_string(),
        };

        // Render TUI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(5),
                    Constraint::Length(3),
                ])
                .split(f.size());

            // Header Banner
            let header = Paragraph::new(" 🔌 AetherX OS - Interactive Serial Monitor & QEMU GDB Portal ")
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Serial Output
            let log_items: Vec<ListItem> = logs.iter().map(|l| ListItem::new(l.clone())).collect();
            let log_list = List::new(log_items)
                .block(Block::default().title(" Real-Time COM1 Serial Output ").borders(Borders::ALL));
            f.render_widget(log_list, chunks[1]);

            // GDB Portal Panel
            let gdb_text = if show_gdb_helper {
                let elf_display = crate::utils::core::paths::WorkspacePaths::find_kernel_elf()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "target/x86_64-unknown-none/debug/aethercore".to_string());
                format!("💡 To Debug: Open another terminal and run:\n  > gdb -ex \"target remote localhost:1234\" -ex \"symbol-file {}\"", elf_display)
            } else {
                "Press 'g' to view GDB attachment instructions | Press 'r' to restart QEMU guest".to_string()
            };
            let gdb_panel = Paragraph::new(gdb_text)
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().title(" QEMU GDB Debugger Portal ").borders(Borders::ALL));
            f.render_widget(gdb_panel, chunks[2]);

            // Footer Status
            let elapsed = start_time.elapsed();
            let footer_content = format!(
                " VM State: {} | Elapsed: {:?} | Press Esc or 'q' to terminate guest.", 
                qemu_status, elapsed
            );
            let footer = Paragraph::new(footer_content)
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[3]);
        })?;

        // Handle keys
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        let _ = qemu_child.kill();
                        break;
                    }
                    KeyCode::Char('g') => {
                        show_gdb_helper = !show_gdb_helper;
                    }
                    KeyCode::Char('r') => {
                        // Restart VM
                        let _ = qemu_child.kill();
                        logs.push("🔄 Restarting QEMU guest...".to_string());
                        let _ = std::fs::remove_file(log_path);
                        let _ = File::create(log_path);
                        log_file = File::open(log_path)?;
                        log_buffer.clear();
                        qemu_child = std::process::Command::new(&qemu_bin)
                            .args(&[
                                "-m", "1024",
                                "-drive", &drive_arg,
                                "-serial", "file:artifacts/qemu_serial.log",
                                "-s",
                                "-display", "none",
                            ])
                            .spawn()
                            .context("Failed to restart QEMU")?;
                    }
                    _ => {}
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}
