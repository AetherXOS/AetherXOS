/// Cgroup Namespace — cgroup root visibility.
///
/// Each cgroup namespace makes the process's current cgroup look like
/// the root of the cgroup hierarchy, hiding parent cgroups.
use super::{alloc_ns_id, NsId};
use alloc::string::String;

/// A single cgroup namespace.
pub struct CgroupNamespace {
    pub id: NsId,
    /// The cgroup path that appears as "/" in this namespace.
    pub root_path: String,
}

impl CgroupNamespace {
    /// Create the root cgroup namespace.
    pub fn root() -> Self {
        Self {
            id: alloc_ns_id(),
            root_path: String::from("/"),
        }
    }

    /// Create a new cgroup namespace with the given root.
    pub fn new() -> Self {
        Self {
            id: alloc_ns_id(),
            root_path: String::from("/"),
        }
    }

    /// Create a cgroup namespace rooted at the given path.
    pub fn with_root(root: String) -> Self {
        Self {
            id: alloc_ns_id(),
            root_path: root,
        }
    }

    /// Translate an absolute cgroup path to namespace-relative.
    pub fn to_relative(&self, abs_path: &str) -> Option<String> {
        if abs_path.starts_with(self.root_path.as_str()) {
            let relative = &abs_path[self.root_path.len()..];
            if relative.is_empty() {
                Some(String::from("/"))
            } else {
                Some(String::from(relative))
            }
        } else {
            None // Path not visible in this namespace.
        }
    }

    /// Translate a namespace-relative path to absolute.
    pub fn to_absolute(&self, rel_path: &str) -> String {
        if self.root_path == "/" {
            String::from(rel_path)
        } else {
            let mut abs = self.root_path.clone();
            if !rel_path.starts_with('/') {
                abs.push('/');
            }
            abs.push_str(rel_path);
            abs
        }
    }
}
