use std::fs::File;
use std::io::Read;
use std::time::Duration;
use anyhow::{Result, bail};
use xmas_elf::ElfFile;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph, List, ListItem},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Modifier},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

pub fn run_analyzer() -> Result<()> {
    let path = match crate::utils::core::paths::WorkspacePaths::find_kernel_elf() {
        Some(p) => p,
        None => bail!("Kernel ELF binary not found. Please build the kernel first."),
    };

    let mut file = File::open(&path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let elf = ElfFile::new(&buffer).map_err(|e| anyhow::anyhow!("Failed to parse ELF: {}", e))?;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
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
            let header = Paragraph::new(format!(" 📊 AetherX OS - Interactive Kernel Bloat Analyzer (Binary: {}) ", path.display()))
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Body Columns
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            // Parse sections
            let mut text_size = 0;
            let mut rodata_size = 0;
            let mut data_size = 0;
            let mut bss_size = 0;
            let mut total_size = 0;

            let mut section_items = Vec::new();
            for sect in elf.section_iter() {
                if sect.get_name(&elf).is_err() { continue; }
                let name = sect.get_name(&elf).unwrap();
                let size = sect.size();
                let addr = sect.address();
                if size == 0 { continue; }

                total_size += size;
                match name {
                    ".text" => text_size += size,
                    ".rodata" => rodata_size += size,
                    ".data" => data_size += size,
                    ".bss" => bss_size += size,
                    _ => {}
                }

                let formatted_size = crate::utils::fs::format::format_size(size);
                let item = ListItem::new(format!("  {} - Size: {} - Addr: {:#x}", name, formatted_size, addr));
                section_items.push(item);
            }

            let section_list = List::new(section_items)
                .block(Block::default().title(" ELF Binary Sections ").borders(Borders::ALL));
            f.render_widget(section_list, body_chunks[0]);

            // Details/Bloat visualizer
            let mut details_items = Vec::new();
            details_items.push(ListItem::new("🔴 Kernel Segment Footprint Summary:"));
            details_items.push(ListItem::new(format!("   - .text (Code Segment):    {} ({:.1}%)", 
                crate::utils::fs::format::format_size(text_size),
                if total_size > 0 { (text_size as f64 / total_size as f64) * 100.0 } else { 0.0 }
            )));
            details_items.push(ListItem::new(format!("   - .rodata (Read-Only):    {} ({:.1}%)", 
                crate::utils::fs::format::format_size(rodata_size),
                if total_size > 0 { (rodata_size as f64 / total_size as f64) * 100.0 } else { 0.0 }
            )));
            details_items.push(ListItem::new(format!("   - .data (Mutable Data):   {} ({:.1}%)", 
                crate::utils::fs::format::format_size(data_size),
                if total_size > 0 { (data_size as f64 / total_size as f64) * 100.0 } else { 0.0 }
            )));
            details_items.push(ListItem::new(format!("   - .bss (Zero Initialized): {} ({:.1}%)", 
                crate::utils::fs::format::format_size(bss_size),
                if total_size > 0 { (bss_size as f64 / total_size as f64) * 100.0 } else { 0.0 }
            )));
            details_items.push(ListItem::new(format!("   - Total Segment Sizes:     {}", 
                crate::utils::fs::format::format_size(total_size)
            )));

            // Add simple visual ASCII progress bars for segment sizing
            let render_bar = |size: u64| -> String {
                let bar_len = 20;
                let ratio = if total_size > 0 { size as f64 / total_size as f64 } else { 0.0 };
                let filled = (ratio * bar_len as f64).round() as usize;
                format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_len - filled))
            };

            details_items.push(ListItem::new(""));
            details_items.push(ListItem::new("📊 Segment Allocation Distribution:"));
            details_items.push(ListItem::new(format!("   .text:   {}", render_bar(text_size))));
            details_items.push(ListItem::new(format!("   .rodata: {}", render_bar(rodata_size))));
            details_items.push(ListItem::new(format!("   .data:   {}", render_bar(data_size))));
            details_items.push(ListItem::new(format!("   .bss:    {}", render_bar(bss_size))));

            let details_list = List::new(details_items)
                .block(Block::default().title(" Segment Breakdown ").borders(Borders::ALL));
            f.render_widget(details_list, body_chunks[1]);

            // Footer
            let footer = Paragraph::new(" Press Enter or Esc to exit the Bloat Analyzer. ")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc => break,
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
