use crate::interfaces::task::TaskId;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex;
use lazy_static::lazy_static;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LandlockAccess {
    WriteFile,
    ReadFile,
    ReadDir,
    RemoveDir,
    RemoveFile,
    MakeChar,
    MakeDir,
    MakeReg,
    MakeSock,
    MakeFifo,
    MakeBlock,
    MakeSym,
    Refer,
}

#[derive(Clone)]
pub struct LandlockRuleset {
    pub rules: BTreeMap<String, BTreeSet<LandlockAccess>>,
}

lazy_static! {
    static ref TASK_RULESETS: Mutex<BTreeMap<TaskId, Arc<LandlockRuleset>>> = Mutex::new(BTreeMap::new());
}

pub fn create_ruleset_internal() -> LandlockRuleset {
    LandlockRuleset {
        rules: BTreeMap::new(),
    }
}

pub fn add_rule(ruleset: &mut LandlockRuleset, path: String, access: BTreeSet<LandlockAccess>) {
    ruleset.rules.insert(path, access);
}

pub fn restrict_self(task_id: TaskId, ruleset: LandlockRuleset) {
    TASK_RULESETS.lock().insert(task_id, Arc::new(ruleset));
}

pub fn check_access(task_id: TaskId, path: &str, access: LandlockAccess) -> bool {
    let map = TASK_RULESETS.lock();
    let ruleset = match map.get(&task_id) {
        Some(r) => r,
        None => return true, // No landlock restriction
    };

    // Check if the path or any of its parents are in the ruleset
    let mut current_path = path;
    loop {
        if let Some(allowed_access) = ruleset.rules.get(current_path) {
            return allowed_access.contains(&access);
        }
        
        if current_path == "/" || current_path.is_empty() {
            break;
        }
        
        // Go up one level
        if let Some(pos) = current_path.rfind('/') {
            current_path = if pos == 0 { "/" } else { &current_path[..pos] };
        } else {
            break;
        }
    }

    false // Denied if not explicitly allowed and ruleset exists
}
