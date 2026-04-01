use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

#[derive(Clone)]
struct SessionRecord {
    session_id: String,
    seat_id: String,
    active: bool,
}

static SESSION_DB: Mutex<Vec<SessionRecord>> = Mutex::new(Vec::new());

pub fn register_session(session_id: &str, seat_id: &str) {
    let mut db = SESSION_DB.lock();
    if db.iter().any(|v| v.session_id == session_id) {
        return;
    }
    db.push(SessionRecord {
        session_id: String::from(session_id),
        seat_id: String::from(seat_id),
        active: false,
    });
}

pub fn mark_session_active(session_id: &str) -> bool {
    let mut db = SESSION_DB.lock();
    if let Some(record) = db.iter_mut().find(|v| v.session_id == session_id) {
        record.active = true;
        return true;
    }
    false
}

pub fn session_snapshot() -> BTreeMap<String, (String, bool)> {
    let db = SESSION_DB.lock();
    let mut out = BTreeMap::new();
    for entry in db.iter() {
        out.insert(entry.session_id.clone(), (entry.seat_id.clone(), entry.active));
    }
    out
}
