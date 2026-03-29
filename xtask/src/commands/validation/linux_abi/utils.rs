//! Shared utilities for Linux ABI analysis.

pub fn extract_fn_body(text: &str, fn_name: &str) -> String {
    let needle = format!("fn {fn_name}(");
    if let Some(start) = text.find(&needle) {
        if let Some(brace_start) = text[start..].find('{') {
            let body_start = start + brace_start;
            let mut depth = 0;
            let mut end = body_start;
            for (i, ch) in text[body_start..].chars().enumerate() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = body_start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            return text[body_start..end].to_string();
        }
    }
    String::new()
}

pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
