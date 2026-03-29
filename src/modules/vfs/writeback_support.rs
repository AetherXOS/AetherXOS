use alloc::collections::BTreeMap;

use super::writeback::{DirtyPageEntry, DirtyPageKey};

pub(super) fn dirty_keys_for_inode(
    dirty_pages: &BTreeMap<DirtyPageKey, DirtyPageEntry>,
    ino: u64,
) -> alloc::vec::Vec<DirtyPageKey> {
    dirty_pages
        .keys()
        .filter(|key| key.ino == ino)
        .copied()
        .collect()
}

pub(super) fn remove_dirty_keys_for_inode(
    dirty_pages: &mut BTreeMap<DirtyPageKey, DirtyPageEntry>,
    ino: u64,
) {
    let keys = dirty_keys_for_inode(dirty_pages, ino);
    for key in &keys {
        dirty_pages.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn dirty_key_helpers_filter_and_remove_only_target_inode_entries() {
        let mut dirty_pages = BTreeMap::new();
        dirty_pages.insert(
            DirtyPageKey {
                ino: 1,
                page_idx: 0,
            },
            DirtyPageEntry {
                dirty_since: 1,
                redirty_count: 1,
            },
        );
        dirty_pages.insert(
            DirtyPageKey {
                ino: 1,
                page_idx: 1,
            },
            DirtyPageEntry {
                dirty_since: 2,
                redirty_count: 1,
            },
        );
        dirty_pages.insert(
            DirtyPageKey {
                ino: 2,
                page_idx: 0,
            },
            DirtyPageEntry {
                dirty_since: 3,
                redirty_count: 1,
            },
        );

        let keys = dirty_keys_for_inode(&dirty_pages, 1);
        assert_eq!(keys.len(), 2);

        remove_dirty_keys_for_inode(&mut dirty_pages, 1);
        assert_eq!(dirty_pages.len(), 1);
        assert!(dirty_pages.contains_key(&DirtyPageKey {
            ino: 2,
            page_idx: 0
        }));
    }
}
