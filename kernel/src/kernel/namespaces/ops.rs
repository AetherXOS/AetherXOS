use core::sync::atomic::Ordering;
use super::types::{CloneFlags, nstype_to_ns_type, namespace_flag_mask, NS_SETNS_CALLS};
use super::registry::{NSSET_TABLE, NSFD_TABLE, NEXT_NSSET_ID, NEXT_NSFD};
use super::set::NsSet;

pub fn namespace_set_by_id(id: u32) -> Option<NsSet> {
    NSSET_TABLE.lock().get(&id).cloned()
}

pub fn ensure_namespace_set(id: u32) {
    let mut table = NSSET_TABLE.lock();
    if table.contains_key(&id) {
        return;
    }
    let root = table.get(&0).cloned().unwrap_or_else(NsSet::init_root);
    table.insert(id, root);
}

pub fn unshare_process_namespaces(current_id: u32, flags_raw: u32) -> Result<u32, &'static str> {
    let supported = namespace_flag_mask();
    if (flags_raw & !supported) != 0 {
        return Err("EINVAL");
    }

    if flags_raw == 0 {
        return Ok(current_id);
    }

    let flags = CloneFlags::from_bits(flags_raw).ok_or("EINVAL")?;

    let mut table = NSSET_TABLE.lock();
    let base = table
        .get(&current_id)
        .cloned()
        .or_else(|| table.get(&0).cloned())
        .unwrap_or_else(NsSet::init_root);

    let child = base.unshare(flags);
    let next = NEXT_NSSET_ID.fetch_add(1, Ordering::Relaxed);
    let id = u32::try_from(next).map_err(|_| "EOVERFLOW")?;
    table.insert(id, child);
    Ok(id)
}

pub fn nsfd_open(ns_set_id: u32) -> i32 {
    let fd = NEXT_NSFD.fetch_add(1, Ordering::Relaxed);
    NSFD_TABLE.lock().insert(fd, ns_set_id);
    fd
}

pub fn nsfd_close(fd: i32) {
    NSFD_TABLE.lock().remove(&fd);
}

pub fn setns_process_namespaces(
    current_id: u32,
    nsfd: i32,
    nstype: u32,
) -> Result<u32, &'static str> {
    let target_set_id = {
        let table = NSFD_TABLE.lock();
        *table.get(&nsfd).ok_or("EBADF")?
    };

    let target_set = {
        let table = NSSET_TABLE.lock();
        table.get(&target_set_id).cloned().ok_or("EINVAL")?
    };

    let new_set = if nstype == 0 {
        target_set
    } else {
        let ns_type = nstype_to_ns_type(nstype).ok_or("EINVAL")?;
        let table = NSSET_TABLE.lock();
        let base = table
            .get(&current_id)
            .cloned()
            .or_else(|| table.get(&0).cloned())
            .ok_or("EINVAL")?;
        drop(table);
        let mut joined = base;
        joined.setns(ns_type, &target_set);
        joined
    };

    let next = NEXT_NSSET_ID.fetch_add(1, Ordering::Relaxed);
    let new_id = u32::try_from(next).map_err(|_| "EOVERFLOW")?;
    NSSET_TABLE.lock().insert(new_id, new_set);

    NS_SETNS_CALLS.fetch_add(1, Ordering::Relaxed);
    Ok(new_id)
}
