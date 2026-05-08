//! Generic bounded-depth symlink resolver
//!
//! Provides a reusable symlink following mechanism with loop detection,
//! used by tmpfs, writable_fs, and other filesystem implementations.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::boxed::Box;
use crate::modules::vfs::constants::SYMLINK_MAX_DEPTH;

/// Callback to resolve a single step in symlink resolution.
///
/// Given a path, should return:
/// - `Ok(None)` if path exists and is not a symlink (resolution complete)
/// - `Ok(Some(target))` if path is a symlink (continue resolution with target)
/// - `Err(e)` if path doesn't exist or error occurred
pub type SymlinkResolver<'a> = Box<dyn Fn(&str) -> Result<Option<String>, &'static str> + 'a>;

/// Resolve a path by following symlinks up to the configured depth limit.
///
/// # Arguments
/// - `path`: Starting path (may be relative or absolute)
/// - `max_depth`: Maximum symlinks to follow (prevents infinite loops)
/// - `resolver_fn`: Callback to check if a path is a symlink and get its target
/// - `join_fn`: Callback to resolve relative symlink targets (e.g., for parent directory)
///
/// # Returns
/// - `Ok(resolved_path)` if symlinks resolved successfully
/// - `Err("ELOOP")` if symlink depth limit exceeded
/// - `Err("ENOENT")` if path not found during resolution
/// - Other `Err` propagated from resolver callback
pub fn resolve_symlinks<F, J>(
    path: &str,
    max_depth: usize,
    resolver_fn: F,
    join_fn: J,
) -> Result<String, &'static str>
where
    F: Fn(&str) -> Result<Option<String>, &'static str>,
    J: Fn(&str, &str) -> String,
{
    let mut current = String::from(path);
    let mut seen = BTreeSet::new();

    for _ in 0..max_depth {
        if !seen.insert(current.clone()) {
            return Err("ELOOP");
        }

        match resolver_fn(&current)? {
            None => return Ok(current), // Not a symlink, we're done
            Some(target) => {
                // Move up one level and resolve target relative to parent
                let parent = if let Some(idx) = current.rfind('/') {
                    String::from(&current[..idx])
                } else {
                    String::new()
                };
                current = join_fn(&parent, &target);
            }
        }
    }

    Err("ELOOP")
}

/// Convenience function for bounded symlink resolution using the default limit.
///
/// See [`resolve_symlinks`] for parameters and return values.
pub fn resolve_symlinks_bounded<F, J>(
    path: &str,
    resolver_fn: F,
    join_fn: J,
) -> Result<String, &'static str>
where
    F: Fn(&str) -> Result<Option<String>, &'static str>,
    J: Fn(&str, &str) -> String,
{
    resolve_symlinks(path, SYMLINK_MAX_DEPTH, resolver_fn, join_fn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use spin::Mutex;

    fn create_test_resolver(symlinks: BTreeMap<String, String>) -> impl Fn(&str) -> Result<Option<String>, &'static str> {
        let symlinks_ref = Mutex::new(symlinks);
        move |path: &str| -> Result<Option<String>, &'static str> {
            if symlinks_ref.lock().contains_key(path) {
                Ok(symlinks_ref.lock().get(path).cloned())
            } else if path.is_empty() || path == "file" || path == "target" {
                Ok(None)
            } else {
                Err("ENOENT")
            }
        }
    }

    #[test_case]
    fn test_no_symlinks() {
        let symlinks: BTreeMap<String, String> = BTreeMap::new();
        let resolver = create_test_resolver(symlinks);
        let join_fn = |_base: &str, target: &str| String::from(target);

        let result = resolve_symlinks_bounded("file", resolver, join_fn);
        assert_eq!(result, Ok("file".to_string()));
    }

    #[test_case]
    fn test_single_symlink() {
        let mut symlinks: BTreeMap<String, String> = BTreeMap::new();
        symlinks.insert("link".to_string(), "target".to_string());
        let resolver = create_test_resolver(symlinks);
        let join_fn = |_base: &str, target: &str| target.to_string();

        let result = resolve_symlinks_bounded("link", resolver, join_fn);
        assert_eq!(result, Ok("target".to_string()));
    }

    #[test_case]
    fn test_loop_detection() {
        let mut symlinks: BTreeMap<String, String> = BTreeMap::new();
        symlinks.insert("a".to_string(), "b".to_string());
        symlinks.insert("b".to_string(), "a".to_string());
        let resolver = create_test_resolver(symlinks);
        let join_fn = |_base: &str, target: &str| target.to_string();

        let result = resolve_symlinks_bounded("a", resolver, join_fn);
        assert_eq!(result, Err("ELOOP"));
    }

    #[test_case]
    fn test_max_depth_exceeded() {
        let mut symlinks: BTreeMap<String, String> = BTreeMap::new();
        for i in 0..20 {
            symlinks.insert(format!("link{}", i), format!("link{}", i + 1));
        }
        symlinks.insert("link20".to_string(), "target".to_string());
        let resolver = create_test_resolver(symlinks);
        let join_fn = |_base: &str, target: &str| target.to_string();

        let result = resolve_symlinks_bounded("link0", resolver, join_fn);
        assert_eq!(result, Err("ELOOP"));
    }
}
