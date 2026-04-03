use super::*;
use crate::modules::vfs::devfs::{DevFs, DeviceMetadata};

/// `/dev/stdin` — reads from serial, writes fail.
pub struct DevStdin;

impl File for DevStdin {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Ok(0)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("EBADF")
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020444,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// `/dev/stdout` — writes to serial.
pub struct DevStdout;

impl File for DevStdout {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("EBADF")
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        for &byte in buf {
            #[cfg(target_arch = "x86_64")]
            {
                crate::hal::serial::SERIAL1.lock().send(byte);
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                let _ = byte;
            }
        }
        Ok(buf.len())
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020222,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn poll_events(&self) -> PollEvents {
        PollEvents::OUT
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// `/dev/stderr` — writes to serial (same as stdout).
pub struct DevStderr;

impl File for DevStderr {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("EBADF")
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        for &byte in buf {
            #[cfg(target_arch = "x86_64")]
            {
                crate::hal::serial::SERIAL1.lock().send(byte);
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                let _ = byte;
            }
        }
        Ok(buf.len())
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020222,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn poll_events(&self) -> PollEvents {
        PollEvents::OUT
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Register all standard Linux special device nodes into a DevFs instance.
pub fn register_linux_special_devices(devfs: &DevFs) {
    let _ = devfs.register_device_with_meta(
        "null",
        Box::new(|_| Box::new(DevNull)),
        DeviceMetadata::char_device(0o666, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "zero",
        Box::new(|_| Box::new(DevZero)),
        DeviceMetadata::char_device(0o666, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "full",
        Box::new(|_| Box::new(DevFull)),
        DeviceMetadata::char_device(0o666, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "random",
        Box::new(|_| Box::new(DevRandom)),
        DeviceMetadata::char_device(0o666, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "urandom",
        Box::new(|_| Box::new(DevRandom)),
        DeviceMetadata::char_device(0o666, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "tty",
        Box::new(|_| Box::new(DevTty::new())),
        DeviceMetadata::char_device(0o666, 0, 5, false),
    );

    let _ = devfs.register_device_with_meta(
        "console",
        Box::new(|_| Box::new(DevTty::new())),
        DeviceMetadata::char_device(0o600, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "stdin",
        Box::new(|_| Box::new(DevStdin)),
        DeviceMetadata::char_device(0o444, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "stdout",
        Box::new(|_| Box::new(DevStdout)),
        DeviceMetadata::char_device(0o222, 0, 0, false),
    );

    let _ = devfs.register_device_with_meta(
        "stderr",
        Box::new(|_| Box::new(DevStderr)),
        DeviceMetadata::char_device(0o222, 0, 0, false),
    );
}
