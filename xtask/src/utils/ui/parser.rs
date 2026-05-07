/// Extract a function body starting from a given function name or offset.
pub fn extract_fn_body(text: &str, function_name: &str) -> Option<String> {
    let marker = format!("fn {}", function_name);
    let start = text.find(&marker)?;
    let rest = text.get(start..)?;
    let open_rel = rest.find('{')?;
    let open_abs = start + open_rel;

    extract_body(text, open_abs)
}

/// Extract a block starting from an opening brace offset.
pub fn extract_body(text: &str, brace_offset: usize) -> Option<String> {
    if !text.is_char_boundary(brace_offset) || !text[brace_offset..].starts_with('{') {
        return None;
    }

    let mut depth = 0usize;
    let mut close_abs = None;
    for (idx, ch) in text[brace_offset..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    close_abs = Some(brace_offset + idx + 1);
                    break;
                }
            }
            _ => {}
        }
    }

    let end = close_abs?;
    text.get(brace_offset..end).map(|s| s.to_string())
}

pub fn parse_generated_bool_const(text: &str, key: &str) -> Option<bool> {
    let needle = format!("pub const {key}: bool = ");
    let line = text.lines().find(|line| line.contains(&needle))?;
    if line.contains("= true;") {
        Some(true)
    } else if line.contains("= false;") {
        Some(false)
    } else {
        None
    }
}

pub fn parse_generated_str_const(text: &str, key: &str) -> Option<String> {
    let needle = format!("pub const {key}: &str = ");
    let line = text.lines().find(|line| line.contains(&needle))?;
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

pub fn parse_default_features(cargo_toml: &str) -> Vec<String> {
    let features_start = match cargo_toml.find("[features]") {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    let features_block = &cargo_toml[features_start..];
    let default_start = match features_block.find("default") {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    let default_block = &features_block[default_start..];
    let list_open = match default_block.find('[') {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    let list_close = match default_block[list_open + 1..].find(']') {
        Some(idx) => idx + list_open + 1,
        None => return Vec::new(),
    };
    default_block[list_open + 1..list_close]
        .split(',')
        .map(str::trim)
        .map(|item| item.trim_matches('"'))
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}
