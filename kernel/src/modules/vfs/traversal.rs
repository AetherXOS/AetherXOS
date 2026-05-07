use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use crate::interfaces::TaskId;
use super::mount_table::MountTable;
use super::constants::SYMLINK_MAX_DEPTH;

pub struct PathTraversal<'a> {
    mount_table: &'a MountTable,
}

impl<'a> PathTraversal<'a> {
    pub fn new(mount_table: &'a MountTable) -> Self {
        Self { mount_table }
    }

    /// Resolve a path component-by-component, following symlinks at each step.
    pub fn resolve_path(
        &self,
        path: &str,
        tid: TaskId,
        follow_last: bool,
    ) -> Result<String, &'static str> {
        let mut current_path = String::from("/");
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut depth = 0;

        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            let next_target = if current_path == "/" {
                format!("/{}", component)
            } else {
                format!("{}/{}", current_path, component)
            };

            // 1. Determine which filesystem owns this path
            let (fs, relative_path) = self.mount_table.resolve_path(&next_target)
                .ok_or("path not found in mount table")?;

            // 2. Check if it's a symlink
            match fs.readlink(&relative_path, tid) {
                Ok(target) => {
                    if is_last && !follow_last {
                        // Return the symlink path itself if we don't want to follow the final link
                        current_path = next_target;
                        continue;
                    }

                    depth += 1;
                    if depth > SYMLINK_MAX_DEPTH {
                        return Err("ELOOP");
                    }

                    // 3. Resolve target (absolute or relative)
                    if target.starts_with('/') {
                        // Start over with absolute target
                        return self.resolve_path(&target, tid, follow_last);
                    } else {
                        // Resolve relative to current directory
                        let mut new_path = current_path.clone();
                        if !new_path.ends_with('/') {
                            new_path.push('/');
                        }
                        new_path.push_str(&target);
                        // We need to recursively resolve the newly formed path from this point
                        // For simplicity in this implementation, we re-evaluate from root for safety
                        // but a production kernel would optimized this.
                        current_path = self.resolve_path(&new_path, tid, true)?;
                    }
                }
                Err("operation not supported") | Err("ENOENT") => {
                    // Not a symlink or doesn't exist, proceed to next component
                    current_path = next_target;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(current_path)
    }
}
