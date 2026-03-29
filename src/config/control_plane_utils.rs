use super::ConfigSetError;
use alloc::string::String;

pub(super) fn normalize_config_key(raw: &str) -> String {
    let trimmed = raw.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        let c = ch.to_ascii_lowercase();
        if matches!(c, '.' | '-' | ' ' | '/' | ':') {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    String::from(out.trim_matches('_'))
}

pub(super) fn normalize_feature_name(raw: &str) -> String {
    let normalized = normalize_config_key(raw);
    if let Some(rest) = normalized.strip_prefix("feature_") {
        return String::from(rest);
    }
    if let Some(rest) = normalized.strip_prefix("feat_") {
        return String::from(rest);
    }
    normalized
}

pub(super) fn parse_feature_flag(raw: &str) -> Result<bool, ConfigSetError> {
    if raw.eq_ignore_ascii_case("true")
        || raw.eq_ignore_ascii_case("1")
        || raw.eq_ignore_ascii_case("yes")
        || raw.eq_ignore_ascii_case("on")
        || raw.eq_ignore_ascii_case("enable")
        || raw.eq_ignore_ascii_case("enabled")
    {
        return Ok(true);
    }
    if raw.eq_ignore_ascii_case("false")
        || raw.eq_ignore_ascii_case("0")
        || raw.eq_ignore_ascii_case("no")
        || raw.eq_ignore_ascii_case("off")
        || raw.eq_ignore_ascii_case("disable")
        || raw.eq_ignore_ascii_case("disabled")
    {
        return Ok(false);
    }
    Err(ConfigSetError::InvalidValue)
}

pub(super) fn is_truthy(raw: &str) -> bool {
    raw.eq_ignore_ascii_case("true")
        || raw.eq_ignore_ascii_case("1")
        || raw.eq_ignore_ascii_case("yes")
        || raw.eq_ignore_ascii_case("on")
}
