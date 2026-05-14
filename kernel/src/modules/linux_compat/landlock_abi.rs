use crate::modules::posix::fs::BoxedFile;
use super::*;
use crate::modules::security::landlock::{self, LandlockRuleset, LandlockAccess};
use crate::modules::vfs::types::{File, FileStats, IoVec, IoVecMut, PollEvents, SeekFrom};
use core::any::Any;
use alloc::sync::Arc;
use spin::Mutex;

pub struct LandlockRulesetFile {
    pub ruleset: Arc<Mutex<LandlockRuleset>>,
}

impl File for LandlockRulesetFile {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn poll_events(&self) -> PollEvents {
        PollEvents::empty()
    }
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

pub fn sys_linux_landlock_create_ruleset(_attr: UserPtr<u8>, _size: usize, flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    
    let ruleset = Arc::new(Mutex::new(landlock::create_ruleset_internal()));
    let file = alloc::boxed::Box::new(LandlockRulesetFile { ruleset });
    
    crate::require_posix_fs!((_attr) => {
        let fs_id = *crate::modules::posix::fs::SHM_FS_ID;
        let fd = crate::modules::posix::fs::register_handle(
            fs_id,
            alloc::string::String::from("landlock-ruleset"),
            Arc::new(Mutex::new(crate::modules::posix::fs::BoxedFile { inner: file })),
            false,
        );
        fd as usize
    })
}

pub fn sys_linux_landlock_add_rule(
    ruleset_fd: Fd,
    rule_type: usize,
    rule_attr: UserPtr<u8>,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    
    crate::require_posix_fs!((ruleset_fd, rule_attr) => {
        let shared = match crate::modules::posix::fs::get_file_description(ruleset_fd.as_u32()) {
            Ok(f) => f,
            Err(e) => return linux_errno(e.code()),
        };
        
        let handle = shared.handle.lock();
        let ruleset_file = if let Some(bf) = handle.as_any().downcast_ref::<BoxedFile>() {
            match bf.inner.as_any().downcast_ref::<LandlockRulesetFile>() {
                Some(rf) => rf,
                None => return linux_errno(crate::modules::posix_consts::errno::EBADF),
            }
        } else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };
        
        if rule_type != 1 { // LANDLOCK_RULE_PATH_BENEATH
            return linux_inval();
        }
        
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct LandlockPathBeneathAttr {
            allowed_access: u64,
            parent_fd: i32,
        }
        
        let attr = match rule_attr.cast::<LandlockPathBeneathAttr>().read() {
            Ok(v) => v,
            Err(e) => return e,
        };
        
        let path = match crate::modules::posix::fs::fd_path(attr.parent_fd as u32) {
            Ok(p) => p,
            Err(e) => return linux_errno(e.code()),
        };
        
        let mut access = alloc::collections::BTreeSet::new();
        if (attr.allowed_access & 0x01) != 0 { access.insert(LandlockAccess::ReadFile); }
        if (attr.allowed_access & 0x02) != 0 { access.insert(LandlockAccess::WriteFile); }
        
        let mut rs = ruleset_file.ruleset.lock();
        landlock::add_rule(&mut rs, path, access);
        0
    })
}

pub fn sys_linux_landlock_restrict_self(ruleset_fd: Fd, flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    
    crate::require_posix_fs!((ruleset_fd) => {
        let shared = match crate::modules::posix::fs::get_file_description(ruleset_fd.as_u32()) {
            Ok(f) => f,
            Err(e) => return linux_errno(e.code()),
        };
        
        let handle = shared.handle.lock();
        let ruleset_file = if let Some(bf) = handle.as_any().downcast_ref::<BoxedFile>() {
            match bf.inner.as_any().downcast_ref::<LandlockRulesetFile>() {
                Some(rf) => rf,
                None => return linux_errno(crate::modules::posix_consts::errno::EBADF),
            }
        } else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };
        
        let tid = crate::interfaces::TaskId(crate::modules::posix::process::gettid());
        landlock::restrict_self(tid, ruleset_file.ruleset.lock().clone());
        0
    })
}
