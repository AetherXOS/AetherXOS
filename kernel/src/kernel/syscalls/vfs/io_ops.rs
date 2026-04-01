use super::*;
#[cfg(feature = "vfs")]
use crate::interfaces::task::TaskId;
#[cfg(feature = "vfs")]
use alloc::boxed::Box;
#[cfg(feature = "vfs")]
use core::sync::atomic::Ordering;

pub(crate) fn sys_vfs_open(_path_ptr: usize, _path_len: usize, _flags: usize) -> usize {
    #[cfg(feature = "vfs")]
    {
        let process = match crate::kernel::launch::current_process_arc() {
            Some(p) => p,
            None => return invalid_arg(),
        };

        with_user_vfs_path(_path_ptr, _path_len, |path| {
            let mount_id = match crate::kernel::vfs_control::mount_id_by_path(b"/") {
                Some(id) => id,
                None => return invalid_arg(),
            };

            let tid = unsafe {
                crate::kernel::cpu_local::CpuLocal::try_get()
                    .map(|cpu| TaskId(cpu.current_task.load(Ordering::Relaxed)))
                    .unwrap_or(TaskId(0))
            };

            let path_str = match core::str::from_utf8(path) {
                Ok(s) => s,
                Err(_) => return invalid_arg(),
            };
            let file = match crate::kernel::vfs_control::ramfs_open_file(mount_id, path_str, tid) {
                Ok(f) => f,
                Err(_) => return invalid_arg(),
            };

            let mut files = process.files.lock();
            let fd = match (3..1024).find(|i| !files.contains_key(i)) {
                Some(id) => id,
                None => return invalid_arg(),
            };
            files.insert(fd, file);
            process.open_file_count.fetch_add(1, Ordering::Relaxed);
            fd
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_vfs_read(_fd: usize, _buf_ptr: usize, _len: usize) -> usize {
    #[cfg(feature = "vfs")]
    {
        let process = match crate::kernel::launch::current_process_arc() {
            Some(p) => p,
            None => return invalid_arg(),
        };

        let mut files = process.files.lock();
        let file: &mut Box<dyn crate::modules::vfs::File> = match files.get_mut(&_fd) {
            Some(f) => f,
            None => return invalid_arg(),
        };

        with_user_write_bytes(_buf_ptr, _len, |buf| match file.read(buf) {
            Ok(n) => n,
            Err(_) => invalid_arg(),
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_vfs_write(_fd: usize, _buf_ptr: usize, _len: usize) -> usize {
    #[cfg(feature = "vfs")]
    {
        let process = match crate::kernel::launch::current_process_arc() {
            Some(p) => p,
            None => return invalid_arg(),
        };

        let mut files = process.files.lock();
        let file: &mut Box<dyn crate::modules::vfs::File> = match files.get_mut(&_fd) {
            Some(f) => f,
            None => return invalid_arg(),
        };

        with_user_read_bytes(_buf_ptr, _len, |buf| match file.write(buf) {
            Ok(n) => n,
            Err(_) => invalid_arg(),
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_vfs_close(_fd: usize) -> usize {
    #[cfg(feature = "vfs")]
    {
        let process = match crate::kernel::launch::current_process_arc() {
            Some(p) => p,
            None => return invalid_arg(),
        };

        let mut files = process.files.lock();
        if files.remove(&_fd).is_some() {
            process.open_file_count.fetch_sub(1, Ordering::Relaxed);
            0
        } else {
            invalid_arg()
        }
    }

    #[cfg(not(feature = "vfs"))]
    {
        invalid_arg()
    }
}
