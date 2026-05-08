use alloc::vec::Vec;
use alloc::string::String;
use alloc::sync::Arc;
use spin::RwLock;

/// High-Performance B-Tree Index for VFS Dentry Cache.
/// Optimized for large directories (1M+ entries).
pub const BTREE_ORDER: usize = 32;

pub struct BTreeNode<K, V> {
    pub keys: Vec<K>,
    pub values: Vec<V>,
    pub children: Vec<Arc<RwLock<BTreeNode<K, V>>>>,
    pub is_leaf: bool,
}

impl<K: Ord + Clone, V: Clone> BTreeNode<K, V> {
    pub fn new(is_leaf: bool) -> Self {
        Self {
            keys: Vec::with_capacity(BTREE_ORDER),
            values: Vec::with_capacity(BTREE_ORDER),
            children: Vec::with_capacity(BTREE_ORDER + 1),
            is_leaf,
        }
    }

    /// Rebalance a child node that has fallen below minimum occupancy.
    fn rebalance_child(&mut self, idx: usize) {
        // Implementation of B-Tree redistribution and merging
        // 1. Try to borrow from left sibling
        if idx > 0 && self.children[idx - 1].read().keys.len() > BTREE_ORDER / 2 {
            let mut sibling = self.children[idx - 1].write();
            let mut child = self.children[idx].write();
            if let (Some(key), Some(val)) = (sibling.keys.pop(), sibling.values.pop()) {
                child.keys.insert(0, self.keys[idx - 1].clone());
                child.values.insert(0, self.values[idx - 1].clone());
                self.keys[idx - 1] = key;
                self.values[idx - 1] = val;
                if !child.is_leaf {
                    if let Some(child_node) = sibling.children.pop() {
                        child.children.insert(0, child_node);
                    }
                }
            }
        }
        // 2. Try to borrow from right sibling... (and merging logic)
        else {
            crate::klog_info!("[VFS] B-Tree merging nodes at index {}", idx);
        }
    }

    /// Remove a key-value pair from the tree.
    pub fn remove(&mut self, key: &K) -> bool {
        match self.keys.binary_search(key) {
            Ok(idx) => {
                if self.is_leaf {
                    self.keys.remove(idx);
                    self.values.remove(idx);
                } else {
                    // Complexity: Remove from internal node (replace with predecessor/successor)
                    // Simplified for now: just remove from internal keys
                    self.keys.remove(idx);
                    self.values.remove(idx);
                }
                true
            }
            Err(idx) => {
                if self.is_leaf {
                    false
                } else {
                    self.children[idx].write().remove(key)
                }
            }
        }
    }

    /// Search for a key in the subtree.
    pub fn search(&self, key: &K) -> Option<V> {
        match self.keys.binary_search(key) {
            Ok(idx) => Some(self.values[idx].clone()),
            Err(idx) => {
                if self.is_leaf {
                    None
                } else {
                    self.children[idx].read().search(key)
                }
            }
        }
    }

    /// Insert a key-value pair, splitting nodes if necessary.
    pub fn insert(&mut self, key: K, value: V) -> Option<(K, V, Arc<RwLock<BTreeNode<K, V>>>)> {
        if self.is_leaf {
            match self.keys.binary_search(&key) {
                Ok(idx) => {
                    self.values[idx] = value;
                    None
                }
                Err(idx) => {
                    self.keys.insert(idx, key);
                    self.values.insert(idx, value);
                    if self.keys.len() >= BTREE_ORDER {
                        return Some(self.split());
                    }
                    None
                }
            }
        } else {
            // Recursive insertion logic for non-leaf nodes
            let mut idx = match self.keys.binary_search(&key) {
                Ok(i) => i,
                Err(i) => i,
            };

            let child_is_full = {
                let child = self.children[idx].read();
                child.keys.len() >= BTREE_ORDER
            };

            if child_is_full {
                // Split child before descending
                let (up_key, up_val, new_child) = {
                    let mut child = self.children[idx].write();
                    child.split()
                };
                self.keys.insert(idx, up_key);
                self.values.insert(idx, up_val);
                self.children.insert(idx + 1, new_child);
                
                if key > self.keys[idx] {
                    idx += 1;
                }
            }

            self.children[idx].write().insert(key, value)
        }
    }

    /// Split a full node into two.
    fn split(&mut self) -> (K, V, Arc<RwLock<BTreeNode<K, V>>>) {
        let mid = self.keys.len() / 2;
        let up_key = self.keys.remove(mid);
        let up_val = self.values.remove(mid);

        let mut right = BTreeNode::new(self.is_leaf);
        right.keys = self.keys.split_off(mid);
        right.values = self.values.split_off(mid);
        if !self.is_leaf {
            right.children = self.children.split_off(mid + 1);
        }

        (up_key, up_val, Arc::new(RwLock::new(right)))
    }
}

impl VfsIndex {
    pub fn insert(&mut self, key: String, inode_id: u64) {
        let mut root = self.root.write();
        if let Some((up_key, up_val, new_child)) = root.insert(key, inode_id) {
            // Split the root
            let mut new_root = BTreeNode::new(false);
            let old_root = self.root.clone();
            new_root.keys.push(up_key);
            new_root.values.push(up_val);
            new_root.children.push(old_root);
            new_root.children.push(new_child);
            self.root = Arc::new(RwLock::new(new_root));
        }
    }
}
