use crate::modules::vfs::types::{File, PollEvents};
use core::any::Any;

pub struct PidFile {
    pub target_pid: usize,
}

impl File for PidFile {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("operation not supported")
    }
    fn poll_events(&self) -> PollEvents {
        // pidfd is readable when the process exits
        if !super::process_exists(self.target_pid) {
            PollEvents::IN
        } else {
            PollEvents::empty()
        }
    }
    
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
