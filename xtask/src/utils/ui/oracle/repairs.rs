/// Decoupled file repair strategies.

/// Applies a repair to a file's content based on the target fix type.
pub fn apply_repair(contents: &str, fix_type: &str) -> Option<String> {
    match fix_type {
        "inject_no_std" => {
            if !contents.contains("#![no_std]") {
                let mut new_contents = "#![no_std]\n".to_string();
                new_contents.push_str(contents);
                Some(new_contents)
            } else {
                None
            }
        }
        "convert_to_core" => {
            Some(contents.replace("use std::", "use core::"))
        }
        "convert_to_spin" => {
            Some(contents.replace("std::sync::Mutex", "spin::Mutex"))
        }
        _ => None,
    }
}
