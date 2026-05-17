use std::{io, time::{Duration, Instant}};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, List, ListItem},
    Terminal,
};
use crossbeam_channel::Receiver;

pub enum HudEvent {
    TaskStarted { index: usize },
    TaskLog { line: String },
    TaskFinished { index: usize, success: bool, reason: Option<String> },
    Finished { success: bool },
}

struct TaskState {
    name: String,
    description: String,
    status: String, // "Pending", "Running", "Success", "Failed"
}

pub fn run_hud(rx: Receiver<HudEvent>, task_names: Vec<(String, String)>) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tasks: Vec<TaskState> = task_names.into_iter().map(|(name, desc)| TaskState {
        name,
        description: desc,
        status: "Pending".to_string(),
    }).collect();

    let mut logs: Vec<String> = Vec::new();
    let mut active_task: Option<usize> = None;
    let start_time = Instant::now();
    let mut finished = false;
    let mut build_success = true;

    // Get TUI theme configuration
    let theme = crate::utils::core::config::get_settings().hud_theme;
    let (header_fg, active_fg, accent_fg) = match theme {
        crate::utils::core::config::HudTheme::Cyberpunk => (Color::Cyan, Color::Magenta, Color::Cyan),
        crate::utils::core::config::HudTheme::Matrix => (Color::Green, Color::LightGreen, Color::Green),
        crate::utils::core::config::HudTheme::Steel => (Color::Blue, Color::LightBlue, Color::Blue),
        crate::utils::core::config::HudTheme::Dracula => (Color::Magenta, Color::Yellow, Color::Magenta),
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
                HudEvent::TaskFinished { index, success, reason } => {
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

        // TUI Render
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.size());

            // Header Banner
            let header = Paragraph::new(format!(" 🚀 AetherX OS - Unified Build Pipeline HUD [{}] ", theme))
                .style(Style::default().fg(header_fg).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(accent_fg)));
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

                let item = ListItem::new(format!("{}{} [{}] - {}", prefix, task.name, task.status, task.description))
                    .style(status_style);
                task_items.push(item);
            }

            let task_list = List::new(task_items)
                .block(Block::default()
                    .title(" Orchestration Sequence ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent_fg)));
            f.render_widget(task_list, body_chunks[0]);

            // Log Console Panel
            let log_lines: Vec<ListItem> = logs.iter().map(|l| ListItem::new(l.clone())).collect();
            let log_list = List::new(log_lines)
                .block(Block::default()
                    .title(" Live Compilation Stream ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent_fg)));
            f.render_widget(log_list, body_chunks[1]);

            // Footer Panel
            let footer_text = if finished {
                if build_success {
                    " 🚀 WORKFLOW COMPLETED SUCCESSFULLY. Press Enter to exit TUI HUD."
                } else {
                    " ❌ WORKFLOW FAILED. Press 'd' to run AI Diagnostics, or Enter to exit."
                }
            } else {
                " Orchestrating pipeline steps..."
            };

            let elapsed = start_time.elapsed();
            let footer = Paragraph::new(format!(" Elapsed: {:?} |{}", elapsed, footer_text))
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(accent_fg)));
            f.render_widget(footer, chunks[2]);
        })?;

        // Handle Exit key in finished state
        if finished {
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if let KeyCode::Enter = key.code {
                        break;
                    }
                    if let KeyCode::Char('d') = key.code {
                        if !build_success {
                            // Find the failed task
                            if let Some(failed_task) = tasks.iter().find(|t| t.status.starts_with("Failed")) {
                                // 1. De-initialize raw TUI screen
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                terminal.show_cursor()?;

                                // 2. Trigger AI Diagnostics
                                println!("\n🔍  Running AI Diagnostics for: {}\n", failed_task.name);
                                let _ = crate::utils::ui::oracle::Oracle::suggest_next(&failed_task.name, false, Some(&failed_task.status));
                                
                                println!("\nPress Enter to return...");
                                let mut input = String::new();
                                let _ = std::io::stdin().read_line(&mut input);
                                
                                return Ok(());
                            }
                        }
                    }
                }
            }
        } else {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}
