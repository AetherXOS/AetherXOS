use super::*;

impl RamFs {
    pub fn mkdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        if key == b"/" {
            return Err("already exists");
        }
        let parent = parent_dir(&key).ok_or("invalid path")?;

        {
            let dirs = self.dirs.lock();
            if !dirs.contains(&parent) {
                return Err("parent not found");
            }
            // check parent permissions
            let meta = self.meta.lock();
            if let Some(p_meta) = meta.get(&parent) {
                if !has_owner_access(tid, p_meta.uid) {
                    return Err("permission denied (parent)");
                }
            }
        }

        if self.dirs.lock().contains(&key)
            || self.files.lock().contains_key(&key)
            || self.symlinks.lock().contains_key(&key)
        {
            return Err("already exists");
        }
        self.dirs.lock().insert(key.clone());
        self.meta
            .lock()
            .insert(key, Self::next_meta(DEFAULT_DIR_MODE, tid));
        Ok(())
    }

    pub fn rmdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        if key == b"/" {
            return Err("permission denied");
        }

        {
            let meta = self.meta.lock();
            let entry = meta.get(&key).ok_or("dir not found")?;
            if !Self::has_owner_access(tid, entry.uid) {
                return Err("permission denied");
            }
        }

        {
            let dirs = self.dirs.lock();
            if !dirs.contains(&key) {
                return Err("dir not found");
            }
            if dirs.iter().any(|entry| is_child_of(entry, &key)) {
                return Err("dir not empty");
            }
        }

        if self
            .files
            .lock()
            .keys()
            .any(|entry| is_child_of(entry, &key))
        {
            return Err("dir not empty");
        }

        if self
            .symlinks
            .lock()
            .keys()
            .any(|entry| Self::is_child_of(entry, &key))
        {
            return Err("dir not empty");
        }

        self.dirs.lock().remove(&key);
        self.meta.lock().remove(&key);
        Ok(())
    }

    pub fn chmod(&self, path: &str, mode: u16, tid: TaskId) -> Result<(), &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        let mut meta = self.meta.lock();
        let entry = meta.get_mut(&key).ok_or("not found")?;
        if !Self::has_owner_access(tid, entry.uid) {
            return Err("permission denied");
        }
        entry.mode = mode;
        Ok(())
    }

    pub fn chown(&self, path: &str, uid: u32, gid: u32, tid: TaskId) -> Result<(), &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        let mut meta = self.meta.lock();
        let entry = meta.get_mut(&key).ok_or("not found")?;
        if !Self::has_owner_access(tid, entry.uid) {
            return Err("permission denied");
        }
        entry.uid = uid;
        entry.gid = gid;
        Ok(())
    }

    pub fn link(&self, old_path: &str, new_path: &str, tid: TaskId) -> Result<(), &'static str> {
        let old_key = normalize_path(old_path).ok_or("invalid path")?;
        let new_key = normalize_path(new_path).ok_or("invalid path")?;
        if new_key == b"/" {
            return Err("already exists");
        }
        let parent = Self::parent_dir(&new_key).ok_or("invalid path")?;

        {
            let dirs = self.dirs.lock();
            if !dirs.contains(&parent) {
                return Err("parent not found");
            }
            // check parent permissions
            let meta = self.meta.lock();
            if let Some(p_meta) = meta.get(&parent) {
                if !Self::has_owner_access(tid, p_meta.uid) {
                    return Err("permission denied (parent)");
                }
            }
        }

        if self.dirs.lock().contains(&old_key) || self.symlinks.lock().contains_key(&old_key) {
            return Err("not supported");
        }

        {
            let meta = self.meta.lock();
            let entry = meta.get(&old_key).ok_or("file not found")?;
            if !Self::has_owner_access(tid, entry.uid) {
                return Err("permission denied");
            }
        }

        if self.files.lock().contains_key(&new_key)
            || self.dirs.lock().contains(&new_key)
            || self.symlinks.lock().contains_key(&new_key)
        {
            return Err("already exists");
        }

        let files = self.files.lock();
        let content = files.get(&old_key).cloned().ok_or("file not found")?;
        drop(files);

        self.files.lock().insert(new_key.clone(), content);

        let mut meta = self.meta.lock();
        let src_meta = meta.get(&old_key).copied().ok_or("file not found")?;
        meta.insert(new_key, src_meta);
        Ok(())
    }

    pub fn rename(&self, old_path: &str, new_path: &str, tid: TaskId) -> Result<(), &'static str> {
        let old_key = normalize_path(old_path).ok_or("invalid path")?;
        let new_key = normalize_path(new_path).ok_or("invalid path")?;
        if old_key == b"/" || new_key == b"/" {
            return Err("invalid path");
        }

        {
            let meta = self.meta.lock();
            let entry = meta.get(&old_key).ok_or("not found")?;
            if !Self::has_owner_access(tid, entry.uid) {
                return Err("permission denied");
            }
        }

        let new_parent = Self::parent_dir(&new_key).ok_or("invalid path")?;
        if !self.dirs.lock().contains(&new_parent) {
            return Err("parent not found");
        }

        {
            let files = self.files.lock();
            let dirs = self.dirs.lock();
            let symlinks = self.symlinks.lock();
            if files.contains_key(&new_key)
                || dirs.contains(&new_key)
                || symlinks.contains_key(&new_key)
            {
                return Err("already exists");
            }
        }

        let mut files = self.files.lock();
        if let Some(content) = files.remove(&old_key) {
            files.insert(new_key.clone(), content);
            let mut meta = self.meta.lock();
            if let Some(m) = meta.remove(&old_key) {
                meta.insert(new_key, m);
            }
            return Ok(());
        }
        drop(files);

        let mut symlinks = self.symlinks.lock();
        if let Some(target) = symlinks.remove(&old_key) {
            symlinks.insert(new_key.clone(), target);
            let mut meta = self.meta.lock();
            if let Some(m) = meta.remove(&old_key) {
                meta.insert(new_key, m);
            }
            return Ok(());
        }
        drop(symlinks);

        let mut dirs = self.dirs.lock();
        if !dirs.contains(&old_key) {
            return Err("not found");
        }

        let old_prefix = [old_key.as_slice(), b"/"].concat();
        let new_prefix = [new_key.as_slice(), b"/"].concat();

        let affected_dirs: Vec<Vec<u8>> = dirs
            .iter()
            .filter(|entry| **entry == old_key || Self::is_child_of(entry, &old_key))
            .cloned()
            .collect();

        for entry in &affected_dirs {
            dirs.remove(entry);
        }
        for entry in affected_dirs {
            if entry == old_key {
                dirs.insert(new_key.clone());
            } else {
                let renamed = [new_prefix.as_slice(), &entry[old_prefix.len()..]].concat();
                dirs.insert(renamed);
            }
        }
        drop(dirs);

        let mut files = self.files.lock();
        let affected_files: Vec<Vec<u8>> = files
            .keys()
            .filter(|entry| Self::is_child_of(entry, &old_key))
            .cloned()
            .collect();
        for entry in affected_files {
            if let Some(content) = files.remove(&entry) {
                let renamed = [new_prefix.as_slice(), &entry[old_prefix.len()..]].concat();
                files.insert(renamed, content);
            }
        }

        let mut symlinks = self.symlinks.lock();
        let affected_symlinks: Vec<Vec<u8>> = symlinks
            .keys()
            .filter(|entry| Self::is_child_of(entry, &old_key))
            .cloned()
            .collect();
        for entry in affected_symlinks {
            if let Some(target) = symlinks.remove(&entry) {
                let renamed = [new_prefix.as_slice(), &entry[old_prefix.len()..]].concat();
                symlinks.insert(renamed, target);
            }
        }

        let mut meta = self.meta.lock();
        let affected_meta: Vec<Vec<u8>> = meta
            .keys()
            .filter(|entry| **entry == old_key || Self::is_child_of(entry, &old_key))
            .cloned()
            .collect();
        for entry in affected_meta {
            if let Some(m) = meta.remove(&entry) {
                let renamed = if entry == old_key {
                    new_key.clone()
                } else {
                    [new_prefix.as_slice(), &entry[old_prefix.len()..]].concat()
                };
                meta.insert(renamed, m);
            }
        }

        Ok(())
    }

    pub fn set_times(
        &self,
        path: &str,
        atime_sec: i64,
        atime_nsec: i32,
        mtime_sec: i64,
        mtime_nsec: i32,
        tid: TaskId,
    ) -> Result<(), &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        let mut meta = self.meta.lock();
        let entry = meta.get_mut(&key).ok_or("not found")?;
        if !Self::has_owner_access(tid, entry.uid) {
            return Err("permission denied");
        }
        entry.atime_sec = atime_sec;
        entry.atime_nsec = atime_nsec;
        entry.mtime_sec = mtime_sec;
        entry.mtime_nsec = mtime_nsec;
        Ok(())
    }

    pub fn symlink(&self, target: &str, link_path: &str, tid: TaskId) -> Result<(), &'static str> {
        let target_key = normalize_path(target).ok_or("invalid path")?;
        let link_key = normalize_path(link_path).ok_or("invalid path")?;
        if link_key == b"/" {
            return Err("already exists");
        }
        let parent = Self::parent_dir(&link_key).ok_or("invalid path")?;

        {
            let dirs = self.dirs.lock();
            if !dirs.contains(&parent) {
                return Err("parent not found");
            }
            let meta = self.meta.lock();
            if let Some(p_meta) = meta.get(&parent) {
                if !Self::has_owner_access(tid, p_meta.uid) {
                    return Err("permission denied (parent)");
                }
            }
        }

        if self.files.lock().contains_key(&link_key)
            || self.dirs.lock().contains(&link_key)
            || self.symlinks.lock().contains_key(&link_key)
        {
            return Err("already exists");
        }
        self.symlinks.lock().insert(link_key.clone(), target_key);
        self.meta
            .lock()
            .insert(link_key, Self::make_meta(DEFAULT_SYMLINK_MODE, tid));
        Ok(())
    }

    pub fn readlink(&self, path: &str, _tid: TaskId) -> Result<String, &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        let symlinks = self.symlinks.lock();
        let target = symlinks.get(&key).ok_or("file not found")?;
        String::from_utf8(target.clone()).map_err(|_| "invalid utf8")
    }

    pub fn list_dir(&self, path: &str, _tid: TaskId) -> Result<Vec<String>, &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        if !self.dirs.lock().contains(&key) {
            return Err("dir not found");
        }

        let mut out = BTreeSet::<String>::new();
        let prefix = if key == b"/" {
            b"/".to_vec()
        } else {
            [key.as_slice(), b"/"].concat()
        };

        {
            let dirs = self.dirs.lock();
            for entry in dirs.iter() {
                if entry == &key || !entry.starts_with(&prefix) {
                    continue;
                }
                let tail = &entry[prefix.len()..];
                if tail.is_empty() || tail.contains(&b'/') {
                    continue;
                }
                let name = core::str::from_utf8(tail).map_err(|_| "invalid utf8")?;
                out.insert(name.to_string());
            }
        }

        {
            let symlinks = self.symlinks.lock();
            for entry in symlinks.keys() {
                if !entry.starts_with(&prefix) {
                    continue;
                }
                let tail = &entry[prefix.len()..];
                if tail.is_empty() || tail.contains(&b'/') {
                    continue;
                }
                let name = core::str::from_utf8(tail).map_err(|_| "invalid utf8")?;
                out.insert(name.to_string());
            }
        }

        {
            let files = self.files.lock();
            for entry in files.keys() {
                if !entry.starts_with(&prefix) {
                    continue;
                }
                let tail = &entry[prefix.len()..];
                if tail.is_empty() || tail.contains(&b'/') {
                    continue;
                }
                let name = core::str::from_utf8(tail).map_err(|_| "invalid utf8")?;
                out.insert(name.to_string());
            }
        }

        Ok(out.into_iter().collect())
    }

    pub fn metadata(
        &self,
        path: &str,
        _tid: TaskId,
    ) -> Result<(u64, bool, bool, u16, u32, u32, u64, i64, i64, i64), &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        let meta = self.meta.lock();
        let attrs = meta.get(&key).copied().ok_or("file not found")?;
        drop(meta);

        let size = if self.dirs.lock().contains(&key) {
            0
        } else if let Some(target) = self.symlinks.lock().get(&key) {
            target.len() as u64
        } else {
            let files = self.files.lock();
            let content = files.get(&key).cloned().ok_or("file not found")?;
            let len = content.lock().len() as u64;
            len
        };

        Ok((
            size,
            self.dirs.lock().contains(&key),
            self.symlinks.lock().contains_key(&key),
            attrs.mode,
            attrs.uid,
            attrs.gid,
            attrs.ino,
            attrs.atime_sec,
            attrs.mtime_sec,
            attrs.ctime_sec,
        ))
    }
}
