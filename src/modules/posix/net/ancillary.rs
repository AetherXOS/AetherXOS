use crate::modules::posix::fs::SharedFile;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

/// Tracks in-flight File Descriptions being passed via SCM_RIGHTS.
pub struct AncillaryPacket {
    pub files: alloc::vec::Vec<Arc<SharedFile>>,
}

lazy_static! {
    /// In-flight ancillary data keyed by (source_fd, target_fd) or a connection ID.
    /// Simplified: keyed by the destination socket's UDP port (which we use for Unix sockets).
    static ref IN_FLIGHT_RIGHTS: Mutex<alloc::collections::BTreeMap<u16, VecDeque<AncillaryPacket>>> =
        Mutex::new(alloc::collections::BTreeMap::new());
}

pub fn push_rights(target_port: u16, files: alloc::vec::Vec<Arc<SharedFile>>) {
    let mut map = IN_FLIGHT_RIGHTS.lock();
    map.entry(target_port)
        .or_insert_with(VecDeque::new)
        .push_back(AncillaryPacket { files });
}

pub fn pop_rights(port: u16) -> Option<AncillaryPacket> {
    let mut map = IN_FLIGHT_RIGHTS.lock();
    map.get_mut(&port)?.pop_front()
}
