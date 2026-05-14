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
        flags: super::types::ResolveFlags,
        base_path: Option<&str>,
    ) -> Result<String, &'static str> {
        let root_path = if flags.contains(super::types::ResolveFlags::IN_ROOT) {
            base_path.unwrap_or("/")
        } else {
            "/"
        };

        let mut current_path = if path.starts_with('/') {
            String::from(root_path)
        } else if let Some(base) = base_path {
            String::from(base)
        } else {
            String::from("/")
        };

        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut depth = 0;

        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            
            if *component == ".." {
                if flags.contains(super::types::ResolveFlags::BENEATH) || flags.contains(super::types::ResolveFlags::IN_ROOT) {
                    if let Some(base) = base_path {
                        if current_path == base || current_path == root_path {
                            return Err("EXDEV"); // Cannot escape beneath/root
                        }
                    }
                }
                // Normal .. logic
                if current_path != "/" && current_path != root_path {
                    if let Some(idx) = current_path.rfind('/') {
                        current_path.truncate(if idx == 0 { 1 } else { idx });
                    }
                }
                continue;
            }

            if *component == "." {
                continue;
            }

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
                    if flags.contains(super::types::ResolveFlags::NO_SYMLINKS) {
                        return Err("ELOOP");
                    }

                    if is_last && !follow_last {
                        current_path = next_target;
                        continue;
                    }

                    depth += 1;
                    if depth > SYMLINK_MAX_DEPTH {
                        return Err("ELOOP");
                    }

                    // 3. Resolve target
                    if target.starts_with('/') {
                        if flags.contains(super::types::ResolveFlags::BENEATH) {
                            return Err("EXDEV"); // Absolute symlink escapes beneath
                        }
                        if flags.contains(super::types::ResolveFlags::IN_ROOT) {
                            // Re-resolve from root_path
                            current_path = self.resolve_path(&target, tid, follow_last, flags, Some(root_path))?;
                        } else {
                            current_path = self.resolve_path(&target, tid, follow_last, flags, base_path)?;
                        }
                    } else {
                        let mut new_path = current_path.clone();
                        if !new_path.ends_with('/') {
                            new_path.push('/');
                        }
                        new_path.push_str(&target);
                        current_path = self.resolve_path(&new_path, tid, true, flags, base_path)?;
                        
                        if flags.contains(super::types::ResolveFlags::BENEATH) {
                            if let Some(base) = base_path {
                                if !current_path.starts_with(base) {
                                    return Err("EXDEV");
                                }
                            }
                        }
                    }
                }
                Err("operation not supported") | Err("ENOENT") => {
                    current_path = next_target;
                }
                Err(e) => return Err(e),
            }
        }

        // Final Landlock check
        let is_write = false; // TODO: Pass is_write to resolve_path
        let access = if is_write {
            crate::modules::security::landlock::LandlockAccess::WriteFile
        } else {
            crate::modules::security::landlock::LandlockAccess::ReadFile
        };
        if !crate::modules::security::landlock::check_access(tid, &current_path, access) {
            return Err("permission denied (landlock)");
        }

        Ok(current_path)
    }
}
