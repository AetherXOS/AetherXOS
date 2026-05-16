use std::{io, time::{Duration, Instant}};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, List, ListItem},
    Terminal, Frame,
};
use sysinfo::{System, CpuRefreshKind, RefreshKind};

pub fn launch() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, Duration::from_millis(250));

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, tick_rate: Duration) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
    );

    loop {
        terminal.draw(|f| ui(f, &mut sys))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            sys.refresh_cpu();
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, sys: &mut System) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Title
    let title = Paragraph::new(" AetherX OS - Nexus Intelligence Dashboard ")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Main Content (CPU & Progress)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // CPU Stats
    let cpus = sys.cpus();
    let mut rows = Vec::new();
    for (i, cpu) in cpus.iter().enumerate() {
        rows.push(Row::new(vec![
            format!("CPU {}", i),
            format!("{:.1}%", cpu.cpu_usage()),
            format!("{:.1} GHz", cpu.frequency() as f32 / 1000.0),
        ]));
    }
    
    let table = Table::new(rows, [Constraint::Percentage(30), Constraint::Percentage(30), Constraint::Percentage(40)])
        .header(Row::new(vec!["Core", "Usage", "Freq"]).style(Style::default().fg(Color::Yellow)))
        .block(Block::default().title(" CPU Telemetry ").borders(Borders::ALL));
    f.render_widget(table, body_chunks[0]);

    // System Info / Progress
    let mut info_items = vec![
        ListItem::new("Status: Ready"),
        ListItem::new("Memory: Stable"),
    ];

    // Add Disk / Network if available
    let networks = sysinfo::Networks::new_with_refreshed_list();
    for (name, data) in &networks {
        info_items.push(ListItem::new(format!("{}: ↑{:.1}MB ↓{:.1}MB", name, data.received() as f32 / 1024.0 / 1024.0, data.transmitted() as f32 / 1024.0 / 1024.0)));
    }

    info_items.push(ListItem::new("Press 'q' to exit dashboard"));

    let info = List::new(info_items)
        .block(Block::default().title(" Engine Insights & Network ").borders(Borders::ALL));
    f.render_widget(info, body_chunks[1]);

    // Footer
    let footer = Paragraph::new(" v0.0.1 | AetherX Build Engine | Sovereign Engineering ")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}
