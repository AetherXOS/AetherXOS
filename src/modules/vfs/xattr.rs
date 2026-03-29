/// Extended Attributes (xattr) — key-value metadata for inodes.
///
/// Supports four standard namespaces:
/// - `user.*`     — user-defined attributes (no privilege required)
/// - `system.*`   — system-managed (e.g. ACLs encoded as xattrs)
/// - `security.*` — security module labels (SELinux, capabilities)
/// - `trusted.*`  — only accessible by privileged processes
///
/// ## Configuration
///
/// | Key                   | Default | Description               |
/// |-----------------------|---------|---------------------------|
/// | `xattr_max_name_len`  | 255     | Maximum attribute name    |
/// | `xattr_max_value_len` | 65536   | Maximum attribute value   |
/// | `xattr_max_per_inode` | 128     | Max xattrs per inode      |
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

// ─── Telemetry ───────────────────────────────────────────────────────

static XATTR_GET_CALLS: AtomicU64 = AtomicU64::new(0);
static XATTR_SET_CALLS: AtomicU64 = AtomicU64::new(0);
static XATTR_REMOVE_CALLS: AtomicU64 = AtomicU64::new(0);
static XATTR_LIST_CALLS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct XattrStats {
    pub get_calls: u64,
    pub set_calls: u64,
    pub remove_calls: u64,
    pub list_calls: u64,
}

pub fn xattr_stats() -> XattrStats {
    XattrStats {
        get_calls: XATTR_GET_CALLS.load(Ordering::Relaxed),
        set_calls: XATTR_SET_CALLS.load(Ordering::Relaxed),
        remove_calls: XATTR_REMOVE_CALLS.load(Ordering::Relaxed),
        list_calls: XATTR_LIST_CALLS.load(Ordering::Relaxed),
    }
}

// ─── Configuration ───────────────────────────────────────────────────

/// Limits for extended attribute operations.
#[derive(Debug, Clone, Copy)]
pub struct XattrConfig {
    pub max_name_len: usize,
    pub max_value_len: usize,
    pub max_per_inode: usize,
}

impl Default for XattrConfig {
    fn default() -> Self {
        Self {
            max_name_len: 255,
            max_value_len: 65536,
            max_per_inode: 128,
        }
    }
}

// ─── Xattr Namespace ────────────────────────────────────────────────

/// Xattr namespace classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrNamespace {
    User,
    System,
    Security,
    Trusted,
}

impl XattrNamespace {
    /// Parse the namespace from an attribute name.
    pub fn from_name(name: &str) -> Option<Self> {
        if name.starts_with("user.") {
            Some(Self::User)
        } else if name.starts_with("system.") {
            Some(Self::System)
        } else if name.starts_with("security.") {
            Some(Self::Security)
        } else if name.starts_with("trusted.") {
            Some(Self::Trusted)
        } else {
            None
        }
    }

    /// Check if a given UID/capability set is allowed to access this namespace.
    pub fn access_check(&self, uid: u32, _has_cap_sys_admin: bool) -> bool {
        match self {
            Self::User => true,
            Self::System => true,       // ACLs — kernel manages access
            Self::Security => uid == 0, // Requires privileged access
            Self::Trusted => uid == 0,  // Requires privileged access
        }
    }
}

// ─── Xattr Error ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrError {
    /// Attribute not found.
    NoData,
    /// Name too long.
    NameTooLong,
    /// Value too large.
    ValueTooLarge,
    /// Too many attributes on this inode.
    NoSpace,
    /// Unknown namespace prefix.
    InvalidNamespace,
    /// Permission denied.
    PermissionDenied,
    /// Attribute already exists (XATTR_CREATE flag).
    AlreadyExists,
    /// Attribute does not exist (XATTR_REPLACE flag).
    DoesNotExist,
}

// ─── Set Flags ───────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Flags for setxattr.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct XattrSetFlags: u32 {
        /// Create only — fail if attribute already exists.
        const XATTR_CREATE  = 0x1;
        /// Replace only — fail if attribute does not exist.
        const XATTR_REPLACE = 0x2;
    }
}

// ─── Per-Inode Xattr Store ──────────────────────────────────────────

/// Per-inode extended attribute store.
pub struct InodeXattrs {
    attrs: BTreeMap<String, Vec<u8>>,
    config: XattrConfig,
}

impl InodeXattrs {
    pub fn new() -> Self {
        Self::with_config(XattrConfig::default())
    }

    pub fn with_config(config: XattrConfig) -> Self {
        Self {
            attrs: BTreeMap::new(),
            config,
        }
    }

    /// Get an extended attribute value.
    pub fn get(&self, name: &str) -> Result<&[u8], XattrError> {
        XATTR_GET_CALLS.fetch_add(1, Ordering::Relaxed);
        self.attrs
            .get(name)
            .map(|v| v.as_slice())
            .ok_or(XattrError::NoData)
    }

    /// Set an extended attribute.
    pub fn set(
        &mut self,
        name: &str,
        value: &[u8],
        flags: XattrSetFlags,
    ) -> Result<(), XattrError> {
        XATTR_SET_CALLS.fetch_add(1, Ordering::Relaxed);

        if name.len() > self.config.max_name_len {
            return Err(XattrError::NameTooLong);
        }
        if value.len() > self.config.max_value_len {
            return Err(XattrError::ValueTooLarge);
        }
        XattrNamespace::from_name(name).ok_or(XattrError::InvalidNamespace)?;

        let exists = self.attrs.contains_key(name);

        if flags.contains(XattrSetFlags::XATTR_CREATE) && exists {
            return Err(XattrError::AlreadyExists);
        }
        if flags.contains(XattrSetFlags::XATTR_REPLACE) && !exists {
            return Err(XattrError::DoesNotExist);
        }

        if !exists && self.attrs.len() >= self.config.max_per_inode {
            return Err(XattrError::NoSpace);
        }

        self.attrs.insert(String::from(name), value.to_vec());
        Ok(())
    }

    /// Remove an extended attribute.
    pub fn remove(&mut self, name: &str) -> Result<(), XattrError> {
        XATTR_REMOVE_CALLS.fetch_add(1, Ordering::Relaxed);
        if self.attrs.remove(name).is_some() {
            Ok(())
        } else {
            Err(XattrError::NoData)
        }
    }

    /// List all attribute names.
    pub fn list(&self) -> Vec<&str> {
        XATTR_LIST_CALLS.fetch_add(1, Ordering::Relaxed);
        self.attrs.keys().map(|k| k.as_str()).collect()
    }

    /// Number of attributes stored.
    pub fn count(&self) -> usize {
        self.attrs.len()
    }
}

// ─── Global Xattr Registry ──────────────────────────────────────────

/// Global registry: inode number → xattr store.
/// Separating xattrs from the Inode struct avoids changing the core type.
pub struct XattrRegistry {
    inodes: Mutex<BTreeMap<u64, InodeXattrs>>,
}

impl XattrRegistry {
    pub const fn new() -> Self {
        Self {
            inodes: Mutex::new(BTreeMap::new()),
        }
    }

    /// Get xattr for an inode. Creates an empty store if absent.
    pub fn get_or_create(&self, ino: u64) -> &Self {
        let mut lock = self.inodes.lock();
        lock.entry(ino).or_insert_with(InodeXattrs::new);
        self
    }

    /// Get an xattr value.
    pub fn getxattr(&self, ino: u64, name: &str) -> Result<Vec<u8>, XattrError> {
        let lock = self.inodes.lock();
        match lock.get(&ino) {
            Some(store) => store.get(name).map(|v| v.to_vec()),
            None => Err(XattrError::NoData),
        }
    }

    /// Set an xattr value.
    pub fn setxattr(
        &self,
        ino: u64,
        name: &str,
        value: &[u8],
        flags: XattrSetFlags,
    ) -> Result<(), XattrError> {
        let mut lock = self.inodes.lock();
        let store = lock.entry(ino).or_insert_with(InodeXattrs::new);
        store.set(name, value, flags)
    }

    /// Remove an xattr.
    pub fn removexattr(&self, ino: u64, name: &str) -> Result<(), XattrError> {
        let mut lock = self.inodes.lock();
        match lock.get_mut(&ino) {
            Some(store) => store.remove(name),
            None => Err(XattrError::NoData),
        }
    }

    /// List xattr names for an inode.
    pub fn listxattr(&self, ino: u64) -> Vec<String> {
        let lock = self.inodes.lock();
        match lock.get(&ino) {
            Some(store) => store.list().into_iter().map(String::from).collect(),
            None => Vec::new(),
        }
    }

    /// Remove all xattrs for a deleted inode.
    pub fn remove_inode(&self, ino: u64) {
        self.inodes.lock().remove(&ino);
    }
}

/// Global xattr registry.
pub static XATTR_REGISTRY: XattrRegistry = XattrRegistry::new();
