/// UTS Namespace — isolated hostname and domain name.
///
/// `sethostname()` / `setdomainname()` inside a UTS namespace do not
/// affect the host or other namespaces.
use super::{alloc_ns_id, NsId};
use alloc::string::String;

/// Maximum hostname length (POSIX HOST_NAME_MAX).
pub const HOST_NAME_MAX: usize = 64;
/// Maximum domain name length.
pub const DOMAIN_NAME_MAX: usize = 64;

/// A single UTS namespace.
pub struct UtsNamespace {
    pub id: NsId,
    hostname: String,
    domainname: String,
}

impl UtsNamespace {
    /// Create the root UTS namespace with default names.
    pub fn root() -> Self {
        Self {
            id: alloc_ns_id(),
            hostname: String::from("aethercore"),
            domainname: String::from("(none)"),
        }
    }

    /// Clone from a parent namespace (copy current names).
    pub fn clone_from(parent: &UtsNamespace) -> Self {
        Self {
            id: alloc_ns_id(),
            hostname: parent.hostname.clone(),
            domainname: parent.domainname.clone(),
        }
    }

    /// Get the hostname.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Set the hostname (truncated to HOST_NAME_MAX).
    pub fn set_hostname(&mut self, name: &str) {
        self.hostname = String::from(&name[..name.len().min(HOST_NAME_MAX)]);
    }

    /// Get the domain name.
    pub fn domainname(&self) -> &str {
        &self.domainname
    }

    /// Set the domain name (truncated to DOMAIN_NAME_MAX).
    pub fn set_domainname(&mut self, name: &str) {
        self.domainname = String::from(&name[..name.len().min(DOMAIN_NAME_MAX)]);
    }
}
