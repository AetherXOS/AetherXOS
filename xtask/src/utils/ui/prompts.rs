use anyhow::Result;
use inquire::{Select, Text};
use crate::utils::config;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use super::orchestrator::MULTI_PROGRESS;

/// Interactively select an item from a list.
pub fn select<'a, T: std::fmt::Display>(prompt: &str, options: &'a [T]) -> Result<&'a T> {
    let non_interactive = config::is_non_interactive();
    if non_interactive {
        return Ok(&options[0]);
    }

    match Select::new(prompt, options.iter().collect::<Vec<_>>())
        .with_page_size(15)
        .with_help_message("Type to filter, ↑↓ to move, Enter to select")
        .prompt()
    {
        Ok(s) => Ok(s),
        Err(e) => Err(anyhow::anyhow!("Interactive selection failed: {}", e)),
    }
}

/// Interactively input a string.
pub fn input(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut t = Text::new(prompt);
    if let Some(d) = default {
        t = t.with_default(d);
    }
    let non_interactive = config::is_non_interactive();
    if non_interactive {
        return Ok(default.unwrap_or_default().to_string());
    }

    match t.prompt() {
        Ok(s) => Ok(s),
        Err(e) => Err(anyhow::anyhow!("Interactive input failed: {}", e)),
    }
}

/// Interactively confirm a yes/no question.
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
    let non_interactive = config::is_non_interactive();
    if non_interactive {
        return Ok(default);
    }

    match inquire::Confirm::new(prompt).with_default(default).prompt() {
        Ok(v) => Ok(v),
        Err(e) => Err(anyhow::anyhow!("Interactive confirmation failed: {}", e)),
    }
}

pub fn spinner(message: &str) -> ProgressBar {
    let pb = MULTI_PROGRESS.add(ProgressBar::new_spinner());
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb
}

pub fn progress(total: u64, message: &str) -> ProgressBar {
    let pb = MULTI_PROGRESS.add(ProgressBar::new(total));
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}",
        )
        .unwrap()
        .progress_chars("━╾─"),
    );
    pb.set_message(message.to_string());
    pb
}
