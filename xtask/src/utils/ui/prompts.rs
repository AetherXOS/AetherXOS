use super::orchestrator::MULTI_PROGRESS;
use crate::utils::config;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{MultiSelect, Select, Text};
use std::time::Duration;

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

/// Interactively choose many items from a list.
pub fn multiselect<T: std::fmt::Display>(
    prompt: &str,
    options: &[T],
    defaults: &[usize],
) -> Result<Vec<usize>> {
    let non_interactive = config::is_non_interactive();
    if non_interactive {
        return Ok(defaults.to_vec());
    }

    let selector = MultiSelect::new(prompt, options.iter().collect::<Vec<_>>())
        .with_page_size(15)
        .with_help_message("Type to filter, Space to toggle, Enter to confirm")
        .with_default(defaults);

    match selector.prompt() {
        Ok(selected) => {
            let selected_ptrs: std::collections::HashSet<*const T> =
                selected.into_iter().map(|item| item as *const T).collect();

            let mut indices = Vec::new();
            for (idx, item) in options.iter().enumerate() {
                let ptr = item as *const T;
                if selected_ptrs.contains(&ptr) {
                    indices.push(idx);
                }
            }
            Ok(indices)
        }
        Err(e) => Err(anyhow::anyhow!("Interactive multi-selection failed: {}", e)),
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
