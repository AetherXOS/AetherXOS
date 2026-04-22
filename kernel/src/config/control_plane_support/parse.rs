use super::super::utils::normalize_config_key;
use super::super::*;
use crate::config::ConfigValueKind;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub(crate) fn split_override_entries(raw: &str) -> Vec<String> {
    raw.split(|ch| matches!(ch, ',' | ';' | '\n' | '\r'))
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect()
}

pub(crate) fn parse_override_entry(
    raw: &str,
) -> Result<(String, Option<String>), (String, ConfigSetError)> {
    if let Some(eq) = raw.find('=') {
        let key = normalize_config_key(&raw[..eq]);
        let value = raw[eq + 1..].trim().to_string();
        if key.is_empty() {
            return Err((String::new(), ConfigSetError::UnknownKey));
        }
        if value.is_empty() {
            return Ok((key, None));
        }
        return Ok((key, Some(value)));
    }

    if let Some(rest) = raw.strip_prefix('!') {
        let key = normalize_config_key(rest);
        if key.is_empty() {
            return Err((String::new(), ConfigSetError::UnknownKey));
        }
        return Ok((key, Some("false".to_string())));
    }

    let normalized = normalize_config_key(raw);
    if normalized.is_empty() {
        return Err((String::new(), ConfigSetError::UnknownKey));
    }

    if let Some(key) = normalized.strip_prefix("reset_") {
        if key.is_empty() {
            return Err((normalized, ConfigSetError::UnknownKey));
        }
        return Ok((key.to_string(), None));
    }
    if let Some(key) = normalized.strip_prefix("unset_") {
        if key.is_empty() {
            return Err((normalized, ConfigSetError::UnknownKey));
        }
        return Ok((key.to_string(), None));
    }
    if let Some(key) = normalized.strip_prefix("no_") {
        if key.is_empty() {
            return Err((normalized, ConfigSetError::UnknownKey));
        }
        return Ok((key.to_string(), Some("false".to_string())));
    }

    match KernelConfig::runtime_config_spec(normalized.as_str()) {
        Some(spec) if spec.value_kind == ConfigValueKind::Bool => {
            Ok((normalized, Some("true".to_string())))
        }
        Some(_) => Err((normalized, ConfigSetError::TypeMismatch)),
        None => Err((normalized, ConfigSetError::UnknownKey)),
    }
}
