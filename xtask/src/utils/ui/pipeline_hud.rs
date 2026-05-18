use crossbeam_channel::Receiver;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{
    io,
    time::{Duration, Instant},
};

pub enum HudEvent {
    TaskStarted {
        index: usize,
    },
    TaskLog {
        line: String,
    },
    TaskFinished {
        index: usize,
        success: bool,
        reason: Option<String>,
    },
    Finished {
        success: bool,
    },
}

pub enum HudResult {
    Exit(anyhow::Result<()>),
    Retry,
}

struct TaskState {
    name: String,
    description: String,
    status: String, // "Pending", "Running", "Success", "Failed"
}

pub fn run_hud(rx: Receiver<HudEvent>, task_names: Vec<(String, String)>) -> HudResult {
    if let Err(e) = enable_raw_mode() {
        return HudResult::Exit(Err(anyhow::anyhow!("Failed to enable raw mode: {}", e)));
    }
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return HudResult::Exit(Err(anyhow::anyhow!(
            "Failed to enter alternate screen: {}",
            e
        )));
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            let _ = disable_raw_mode();
            return HudResult::Exit(Err(anyhow::anyhow!("Failed to create terminal: {}", e)));
        }
    };

    let mut tasks: Vec<TaskState> = task_names
        .into_iter()
        .map(|(name, desc)| TaskState {
            name,
            description: desc,
            status: "Pending".to_string(),
        })
        .collect();

    let mut logs: Vec<String> = Vec::new();
    let mut active_task: Option<usize> = None;
    let start_time = Instant::now();
    let mut finished = false;
    let mut build_success = true;

    // Initialize sysinfo telemetry
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut tick_counter = 0;

    // Get TUI theme configuration
    let theme = crate::utils::core::config::get_settings().hud_theme;
    let (header_fg, active_fg, accent_fg) = match theme {
        crate::utils::core::config::HudTheme::Cyberpunk => {
            (Color::Cyan, Color::Magenta, Color::Cyan)
        }
        crate::utils::core::config::HudTheme::Matrix => {
            (Color::Green, Color::LightGreen, Color::Green)
        }
        crate::utils::core::config::HudTheme::Steel => (Color::Blue, Color::LightBlue, Color::Blue),
        crate::utils::core::config::HudTheme::Dracula => {
            (Color::Magenta, Color::Yellow, Color::Magenta)
        }
    };

    loop {
        // Handle incoming events from the channel
        while let Ok(event) = rx.try_recv() {
            match event {
                HudEvent::TaskStarted { index } => {
                    active_task = Some(index);
                    tasks[index].status = "Running".to_string();
                }
                HudEvent::TaskLog { line } => {
                    if logs.len() > 30 {
                        logs.remove(0);
                    }
                    logs.push(line);
                }
                HudEvent::TaskFinished {
                    index,
                    success,
                    reason,
                } => {
                    if success {
                        tasks[index].status = "Success".to_string();
                    } else {
                        tasks[index].status = format!("Failed: {}", reason.unwrap_or_default());
                        build_success = false;
                    }
                }
                HudEvent::Finished { success } => {
                    finished = true;
                    build_success = success;
                }
            }
        }

        // Refresh hardware CPU/RAM status every 500ms (10 * 50ms)
        tick_counter += 1;
        if tick_counter >= 10 {
            sys.refresh_cpu();
            sys.refresh_memory();
            tick_counter = 0;
        }

        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();

        // Build CPU bar: e.g. [████░░░░]
        let cpu_bar_len = 8;
        let cpu_filled =
            (((cpu_usage / 100.0) * cpu_bar_len as f32).round() as usize).min(cpu_bar_len);
        let cpu_bar = format!(
            "[{}{}] {:.0}%",
            "█".repeat(cpu_filled),
            "░".repeat(cpu_bar_len - cpu_filled),
            cpu_usage
        );

        // Build RAM bar: e.g. [██░░░░░░]
        let mem_bar_len = 8;
        let mem_ratio = if total_mem > 0 {
            used_mem as f64 / total_mem as f64
        } else {
            0.0
        };
        let mem_filled = ((mem_ratio * mem_bar_len as f64).round() as usize).min(mem_bar_len);
        let mem_bar = format!(
            "[{}{}] {}/{}",
            "█".repeat(mem_filled),
            "░".repeat(mem_bar_len - mem_filled),
            crate::utils::fs::format::format_size(used_mem),
            crate::utils::fs::format::format_size(total_mem)
        );

        // TUI Render
        let render_res = terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.size());

            // Header Banner
            let header = Paragraph::new(format!(
                " 🚀 AetherX OS - Unified Build Pipeline HUD [{}] ",
                theme
            ))
            .style(Style::default().fg(header_fg).add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent_fg)),
            );
            f.render_widget(header, chunks[0]);

            // Body Columns
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[1]);

            // Task List Panel
            let mut task_items = Vec::new();
            for (idx, task) in tasks.iter().enumerate() {
                let prefix = if active_task == Some(idx) {
                    "▶ "
                } else {
                    "  "
                };

                let status_style = match task.status.as_str() {
                    "Success" => Style::default().fg(Color::Green),
                    "Running" => Style::default().fg(active_fg).add_modifier(Modifier::BOLD),
                    s if s.starts_with("Failed") => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::DarkGray),
                };

                let item = ListItem::new(format!(
                    "{}{} [{}] - {}",
                    prefix, task.name, task.status, task.description
                ))
                .style(status_style);
                task_items.push(item);
            }

            let task_list = List::new(task_items).block(
                Block::default()
                    .title(" Orchestration Sequence ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent_fg)),
            );
            f.render_widget(task_list, body_chunks[0]);

            // Log Console Panel
            let log_lines: Vec<ListItem> = logs.iter().map(|l| ListItem::new(l.clone())).collect();
            let log_list = List::new(log_lines).block(
                Block::default()
                    .title(" Live Compilation Stream ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent_fg)),
            );
            f.render_widget(log_list, body_chunks[1]);

            // Footer Panel
            let footer_text = if finished {
                if build_success {
                    " 🚀 WORKFLOW COMPLETED SUCCESSFULLY. Press Enter to exit."
                } else {
                    " ❌ WORKFLOW FAILED. Press 'd' to run AI Diagnostics, or Enter to exit."
                }
            } else {
                " Orchestrating pipeline steps..."
            };

            let elapsed = start_time.elapsed();
            let footer_content = format!(
                " Elapsed: {:?} | CPU: {} | RAM: {} |{}",
                elapsed, cpu_bar, mem_bar, footer_text
            );
            let footer = Paragraph::new(footer_content)
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(accent_fg)),
                );
            f.render_widget(footer, chunks[2]);
        });

        if let Err(e) = render_res {
            let _ = disable_raw_mode();
            let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
            return HudResult::Exit(Err(anyhow::anyhow!("Failed to draw TUI frame: {}", e)));
        }

        // Handle Exit key in finished state
        if finished {
            let mut key_event = None;
            if let Ok(true) = event::poll(Duration::from_millis(50)) {
                if let Ok(Event::Key(key)) = event::read() {
                    key_event = Some(key.code);
                }
            }

            if let Some(code) = key_event {
                if let KeyCode::Enter = code {
                    break;
                }
                if let KeyCode::Char('d') = code {
                    if !build_success {
                        // Find the failed task
                        if let Some(failed_task) =
                            tasks.iter().find(|t| t.status.starts_with("Failed"))
                        {
                            // 1. De-initialize raw TUI screen
                            let _ = disable_raw_mode();
                            let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
                            let _ = terminal.show_cursor();

                            // 2. Trigger AI Diagnostics
                            println!("\n🔍  Running AI Diagnostics for: {}\n", failed_task.name);
                            match crate::utils::ui::oracle::Oracle::suggest_next(
                                &failed_task.name,
                                false,
                                Some(&failed_task.status),
                            ) {
                                Ok(true) => return HudResult::Retry,
                                Ok(false) => {
                                    return HudResult::Exit(Err(anyhow::anyhow!(
                                        "Build failed at step '{}': {}",
                                        failed_task.name,
                                        failed_task.status
                                    )));
                                }
                                Err(e) => return HudResult::Exit(Err(e)),
                            }
                        }
                    }
                }
            }
        } else {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    if build_success {
        HudResult::Exit(Ok(()))
    } else {
        HudResult::Exit(Err(anyhow::anyhow!("Pipeline execution failed")))
    }
}
