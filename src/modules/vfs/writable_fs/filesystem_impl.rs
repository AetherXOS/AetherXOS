use super::*;

impl<Base: FileSystem> FileSystem for WritableOverlayFs<Base> {
    fn open(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let norm = Self::normalize(path);

        // Check whiteout
        if self.is_whiteout(&norm) {
            return Err("file not found (deleted)");
        }

        // In overlay?
        if self.overlay_exists(&norm) {
            let ino = self
                .path_to_ino
                .lock()
                .get(&norm)
                .copied()
                .ok_or("internal: overlay entry without ino")?;
            let inode = GLOBAL_INODE_CACHE
                .get(ino)
                .ok_or("internal: overlay ino not in cache")?;
            let size = self.entries.lock().get(&norm).map(|e| e.size).unwrap_or(0);
            return Ok(Box::new(OverlayFile {
                ino,
                inode,
                cursor: 0,
                size,
                mount_id: self.mount_id,
            }));
        }

        // Try base FS ÔÇö if found, copy-up for potential future writes
        match self.base.open(path, tid) {
            Ok(f) => Ok(f), // Read-only access through base is fine
            Err(e) => Err(e),
        }
    }

    fn create(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let norm = Self::normalize(path);

        // If whited-out, remove the whiteout
        {
            let mut entries = self.entries.lock();
            if let Some(entry) = entries.get_mut(&norm) {
                if entry.whiteout {
                    entry.whiteout = false;
                    entry.size = 0;
                    let ino = entry.ino;
                    drop(entries);
                    // Clear the page cache
                    if let Some(inode) = GLOBAL_INODE_CACHE.get(ino) {
                        inode.pages.lock().clear();
                    }
                    return self.open(path, tid);
                }
            }
        }

        // Already exists in overlay? Truncate and return.
        if self.overlay_exists(&norm) {
            let mut file = self.open(path, tid)?;
            file.truncate(0)?;
            return Ok(file);
        }

        // Create new entry in overlay
        let ino = alloc_ino();
        let entry = OverlayEntry::new_file(ino, 0o644);
        let inode = Arc::new(Inode::new(ino, entry.mode));
        GLOBAL_INODE_CACHE.insert(inode.clone());
        writeback::register_inode(ino, self.mount_id);

        // Journal the creation
        let parent = Self::parent_path(path);
        let name = Self::basename(path);
        let mut txn = JournalTransaction::new();
        if let Some(ref pp) = parent {
            txn.add(JournalOp::DentryCreate {
                parent_ino: self.path_to_ino.lock().get(pp).copied().unwrap_or(0),
                name_hash: simple_hash(name),
                child_ino: ino,
            });
        }
        txn.add(JournalOp::InodeUpdate {
            ino,
            new_size: 0,
            new_mode: entry.mode,
        });
        // Commit journal (best-effort ÔÇö if no sink, that's OK for in-memory)
        let _ = txn.commit(&RamJournalSink);

        self.entries.lock().insert(norm.clone(), entry);
        self.path_to_ino.lock().insert(norm.clone(), ino);

        // Track in parent's children
        if let Some(pp) = parent {
            let mut dc = self.dir_children.lock();
            dc.entry(pp).or_insert_with(Vec::new).push(name.to_string());
        }

        Ok(Box::new(OverlayFile {
            ino,
            inode,
            cursor: 0,
            size: 0,
            mount_id: self.mount_id,
        }))
    }

    fn remove(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        let norm = Self::normalize(path);

        // If in overlay, mark as whiteout
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get_mut(&norm) {
            entry.whiteout = true;
            // Free inode from cache
            GLOBAL_INODE_CACHE.evict(entry.ino);
            return Ok(());
        }
        drop(entries);

        // Check if exists in base (create a whiteout to hide it)
        if self.base.stat(path, tid).is_ok() {
            let ino = alloc_ino();
            let mut entry = OverlayEntry::new_file(ino, 0o644);
            entry.whiteout = true;
            self.entries.lock().insert(norm, entry);
            return Ok(());
        }

        Err("file not found")
    }

    fn mkdir(&self, path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let norm = Self::normalize(path);

        if self.overlay_exists(&norm) {
            return Err("directory already exists");
        }

        let ino = alloc_ino();
        let entry = OverlayEntry::new_dir(ino, 0o755);
        let inode = Arc::new(Inode::new(ino, entry.mode));
        GLOBAL_INODE_CACHE.insert(inode);
        writeback::register_inode(ino, self.mount_id);

        self.entries.lock().insert(norm.clone(), entry);
        self.path_to_ino.lock().insert(norm.clone(), ino);
        self.dir_children.lock().insert(norm.clone(), Vec::new());

        // Add to parent
        if let Some(pp) = Self::parent_path(path) {
            let name = Self::basename(path);
            self.dir_children
                .lock()
                .entry(pp)
                .or_insert_with(Vec::new)
                .push(name.to_string());
        }

        Ok(())
    }

    fn rmdir(&self, path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let norm = Self::normalize(path);

        // Check if directory is empty in overlay
        let children = self.dir_children.lock();
        if let Some(kids) = children.get(&norm) {
            if !kids.is_empty() {
                return Err("directory not empty");
            }
        }
        drop(children);

        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get_mut(&norm) {
            if !entry.is_dir() {
                return Err("not a directory");
            }
            entry.whiteout = true;
            GLOBAL_INODE_CACHE.evict(entry.ino);
            return Ok(());
        }

        Err("directory not found")
    }

    fn readdir(&self, path: &str, tid: TaskId) -> Result<Vec<DirEntry>, &'static str> {
        let norm = Self::normalize(path);
        let mut result = Vec::new();

        // Collect from base FS (if path not whited-out)
        if !self.is_whiteout(&norm) {
            if let Ok(base_entries) = self.base.readdir(path, tid) {
                for entry in base_entries {
                    // Skip whited-out entries
                    let child_path = if norm == "/" {
                        alloc::format!("/{}", entry.name)
                    } else {
                        alloc::format!("{}/{}", norm, entry.name)
                    };
                    if !self.is_whiteout(&child_path) {
                        result.push(entry);
                    }
                }
            }
        }

        // Merge overlay entries
        if let Some(children) = self.dir_children.lock().get(&norm) {
            for child_name in children {
                let child_path = if norm == "/" {
                    alloc::format!("/{}", child_name)
                } else {
                    alloc::format!("{}/{}", norm, child_name)
                };
                if self.is_whiteout(&child_path) {
                    continue;
                }
                // Skip if already from base
                if result.iter().any(|e| e.name == *child_name) {
                    continue;
                }
                if let Some(entry) = self.entries.lock().get(&child_path) {
                    let kind = if entry.is_dir() {
                        DT_DIR
                    } else if entry.is_symlink() {
                        DT_LNK
                    } else {
                        DT_REG
                    };
                    result.push(DirEntry {
                        name: child_name.clone(),
                        ino: entry.ino,
                        kind,
                    });
                }
            }
        }

        Ok(result)
    }

    fn stat(&self, path: &str, tid: TaskId) -> Result<FileStats, &'static str> {
        let norm = Self::normalize(path);

        if self.is_whiteout(&norm) {
            return Err("file not found");
        }

        // Check overlay first
        if let Some(entry) = self.entries.lock().get(&norm) {
            if !entry.whiteout {
                return Ok(entry.to_stats());
            }
        }

        // Fall through to base
        self.base.stat(path, tid)
    }

    fn chmod(&self, path: &str, mode: u16, tid: TaskId) -> Result<(), &'static str> {
        let norm = Self::normalize(path);
        let mut entries = self.entries.lock();

        if let Some(entry) = entries.get_mut(&norm) {
            if entry.whiteout {
                return Err("file not found");
            }
            entry.mode = (entry.mode & 0o170000) | (mode & 0o7777);
            return Ok(());
        }
        drop(entries);

        // Copy-up from base, then modify
        self.copy_up(path, tid)?;
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get_mut(&norm) {
            entry.mode = (entry.mode & 0o170000) | (mode & 0o7777);
        }
        Ok(())
    }

    fn chown(&self, path: &str, uid: u32, gid: u32, tid: TaskId) -> Result<(), &'static str> {
        let norm = Self::normalize(path);
        let mut entries = self.entries.lock();

        if let Some(entry) = entries.get_mut(&norm) {
            if entry.whiteout {
                return Err("file not found");
            }
            entry.uid = uid;
            entry.gid = gid;
            return Ok(());
        }
        drop(entries);

        self.copy_up(path, tid)?;
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get_mut(&norm) {
            entry.uid = uid;
            entry.gid = gid;
        }
        Ok(())
    }

    fn rename(&self, old_path: &str, new_path: &str, tid: TaskId) -> Result<(), &'static str> {
        let old_norm = Self::normalize(old_path);
        let new_norm = Self::normalize(new_path);

        // Copy-up if needed
        if !self.overlay_exists(&old_norm) {
            self.copy_up(old_path, tid)?;
        }

        let mut entries = self.entries.lock();
        if let Some(mut entry) = entries.remove(&old_norm) {
            entry.whiteout = false;
            let ino = entry.ino;
            entries.insert(new_norm.clone(), entry);
            drop(entries);

            // Create whiteout for old path (in case base has it)
            let mut entries2 = self.entries.lock();
            let mut wo = OverlayEntry::new_file(alloc_ino(), 0o644);
            wo.whiteout = true;
            entries2.insert(old_norm, wo);
            drop(entries2);

            // Update ino mapping
            let mut ino_map = self.path_to_ino.lock();
            ino_map.remove(old_path);
            ino_map.insert(new_norm, ino);

            Ok(())
        } else {
            Err("source not found")
        }
    }

    fn link(&self, old_path: &str, new_path: &str, tid: TaskId) -> Result<(), &'static str> {
        let old_norm = Self::normalize(old_path);
        let new_norm = Self::normalize(new_path);

        // Get the inode for the source
        let ino = if let Some(ino) = self.get_or_create_inode(&old_norm) {
            ino
        } else {
            self.copy_up(old_path, tid)?
        };

        // Create a new entry pointing to the same inode
        let entries = self.entries.lock();
        if let Some(src) = entries.get(&old_norm) {
            let mut new_entry = src.clone();
            new_entry.link_count += 1;
            drop(entries);
            self.entries.lock().insert(new_norm.clone(), new_entry);
            self.path_to_ino.lock().insert(new_norm, ino);
            Ok(())
        } else {
            Err("source not found")
        }
    }

    fn symlink(&self, target: &str, link_path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let norm = Self::normalize(link_path);
        let ino = alloc_ino();
        let entry = OverlayEntry::new_symlink(ino, target.to_string());

        self.entries.lock().insert(norm.clone(), entry);
        self.path_to_ino.lock().insert(norm, ino);
        Ok(())
    }

    fn readlink(&self, path: &str, tid: TaskId) -> Result<String, &'static str> {
        let norm = Self::normalize(path);

        // Check overlay
        if let Some(entry) = self.entries.lock().get(&norm) {
            if entry.is_symlink() {
                return entry.symlink_target.clone().ok_or("not a symlink");
            }
        }

        // Fall through to base
        self.base.readlink(path, tid)
    }

    fn set_times(
        &self,
        path: &str,
        atime: u64,
        mtime: u64,
        tid: TaskId,
    ) -> Result<(), &'static str> {
        let norm = Self::normalize(path);

        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get_mut(&norm) {
            entry.atime = atime;
            entry.mtime = mtime;
            return Ok(());
        }
        drop(entries);

        self.copy_up(path, tid)?;
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get_mut(&norm) {
            entry.atime = atime;
            entry.mtime = mtime;
        }
        Ok(())
    }

    fn sync_fs(&self) -> Result<(), &'static str> {
        // Flush all dirty pages for this mount
        writeback::sync_all()?;
        Ok(())
    }
}

// ÔöÇÔöÇ Helpers ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

// Dummy journal sink for in-memory operation.
