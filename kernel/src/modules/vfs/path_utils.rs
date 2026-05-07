//! Shared path utilities for filesystem operations
//!
//! Consolidates common path manipulation operations (normalize, parent, join, etc.)
//! used across tmpfs, writable_fs, ramfs, and other filesystem implementations.

use alloc::string::String;
use alloc::format;

/// Normalize a path by trimming slashes and returning the canonical form.
///
/// # Examples
/// - `/foo/` → `foo`
/// - `//foo//bar//` → `foo/bar`
/// - `/` or `` → `` (empty string for root)
pub fn normalize(path: &str) -> String {
    String::from(path.trim_matches('/'))
}

/// Get the parent directory of a path.
///
/// # Returns
/// - `Some(parent)` for non-root paths
/// - `None` for root path
/// - For `/foo`, returns `Some("")` (empty = root)
///
/// # Examples
/// - `/foo/bar` → `Some("foo")`
/// - `/foo` → `Some("")`
/// - `/` or `` → `None`
pub fn parent(path: &str) -> Option<String> {
    let normalized = normalize(path);
    if normalized.is_empty() {
        return None;
    }
    match normalized.rfind('/') {
        Some(idx) => Some(String::from(&normalized[..idx])),
        None => Some(String::new()), // parent is root
    }
}

/// Get the basename (final component) of a path.
///
/// # Examples
/// - `/foo/bar/baz` → `baz`
/// - `/foo` → `foo`
/// - `/foo/` → `foo` (trailing slash stripped)
/// - `/` or `` → `` (empty)
pub fn basename(path: &str) -> String {
    let normalized = normalize(path);
    match normalized.rfind('/') {
        Some(idx) => String::from(&normalized[idx + 1..]),
        None => normalized,
    }
}

/// Join a relative path to a base directory path.
///
/// If `target` is absolute (starts with `/`), returns the normalized target.
/// Otherwise, joins `base` and `target` with `/`.
///
/// # Examples
/// - `join_relative("foo", "bar")` → `"foo/bar"`
/// - `join_relative("", "bar")` → `"bar"`
/// - `join_relative("foo", "/bar")` → `"bar"` (absolute target)
/// - `join_relative("foo/baz", "../bar")` → `"foo/baz/../bar"` (no normalization)
pub fn join_relative(base: &str, target: &str) -> String {
    if target.starts_with('/') {
        return normalize(target);
    }
    if base.is_empty() {
        normalize(target)
    } else {
        normalize(&format!("{base}/{target}"))
    }
}

/// Resolve a relative path component within a base directory.
///
/// Handles `.` (current), `..` (parent), and normal path components.
/// Does not follow symlinks or check filesystem.
///
/// # Examples
/// - `resolve_relative("foo/bar", "..", "")` → `"foo"` (with baz → foo/bar/../ → foo)
/// - `resolve_relative("foo/bar", ".", "")` → `"foo/bar"`
/// - `resolve_relative("foo", "baz", "")` → `"foo/baz"`
pub fn resolve_relative(base: &str, component: &str, _unused: &str) -> String {
    let normalized_base = normalize(base);

    match component {
        "" | "." => normalized_base,
        ".." => {
            if normalized_base.is_empty() {
                String::new() // Already at root
            } else {
                match normalized_base.rfind('/') {
                    Some(idx) => String::from(&normalized_base[..idx]),
                    None => String::new(), // Move up to root
                }
            }
        }
        _ => {
            if normalized_base.is_empty() {
                normalize(component)
            } else {
                normalize(&format!("{normalized_base}/{component}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_normalize() {
        assert_eq!(normalize("/foo/bar"), "foo/bar");
        assert_eq!(normalize("//foo//bar//"), "foo/bar");
        assert_eq!(normalize("/"), "");
        assert_eq!(normalize(""), "");
    }

    #[test_case]
    fn test_parent() {
        assert_eq!(parent("/foo/bar"), Some(String::from("foo")));
        assert_eq!(parent("/foo"), Some(String::from("")));
        assert_eq!(parent("/"), None);
        assert_eq!(parent(""), None);
    }

    #[test_case]
    fn test_basename() {
        assert_eq!(basename("/foo/bar/baz"), "baz");
        assert_eq!(basename("/foo"), "foo");
        assert_eq!(basename("/"), "");
        assert_eq!(basename(""), "");
    }

    #[test_case]
    fn test_join_relative() {
        assert_eq!(join_relative("foo", "bar"), "foo/bar");
        assert_eq!(join_relative("", "bar"), "bar");
        assert_eq!(join_relative("foo", "/bar"), "bar");
        assert_eq!(join_relative("foo/baz", "../bar"), "foo/baz/../bar");
    }

    #[test_case]
    fn test_resolve_relative() {
        assert_eq!(resolve_relative("foo/bar", "..", ""), "foo");
        assert_eq!(resolve_relative("foo/bar", ".", ""), "foo/bar");
        assert_eq!(resolve_relative("foo", "baz", ""), "foo/baz");
        assert_eq!(resolve_relative("", "baz", ""), "baz");
    }
}
