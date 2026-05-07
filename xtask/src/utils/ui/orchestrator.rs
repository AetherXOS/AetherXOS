use indicatif::MultiProgress;
use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
    pub static ref MULTI_PROGRESS: MultiProgress = MultiProgress::new();
    pub static ref UI_ACTIVE: RwLock<bool> = RwLock::new(false);
}

pub fn set_ui_active(active: bool) {
    if let Ok(mut lock) = UI_ACTIVE.write() {
        *lock = active;
    }
}

pub fn is_ui_active() -> bool {
    UI_ACTIVE.read().map(|lock| *lock).unwrap_or(false)
}
