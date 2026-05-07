use crate::modules::posix::fs::{get_file_description, register_file_description};
use crate::modules::vfs::types::File;
use crate::modules::network::types::{Packet, PacketData};
use alloc::sync::Arc;
use spin::Mutex;

/// Zero-Copy sendfile(2) implementation.
/// Bridges the VFS Page Cache directly to the Network Stack.
pub fn sys_sendfile(
    out_fd: u32,
    in_fd: u32,
    offset_ptr: *mut i64,
    count: usize,
) -> Result<usize, &'static str> {
    let out_desc = get_file_description(out_fd).map_err(|_| "EBADF")?;
    let in_desc = get_file_description(in_fd).map_err(|_| "EBADF")?;

    // Validate inputs
    // For simplicity, we assume out_fd is a socket and in_fd is a regular file.
    
    let mut offset = if !offset_ptr.is_null() {
        unsafe { *offset_ptr as u64 }
    } else {
        in_desc.handle.lock().seek(crate::modules::vfs::SeekFrom::Current(0))?
    };

    // 1. Get physical frames from the source file (Zero-Copy)
    let frames = in_desc.handle.lock().mmap_physical(offset, count)?;
    
    let mut total_sent = 0;
    for &frame_addr in &frames {
        let chunk = core::cmp::min(count - total_sent, 4096);
        if chunk == 0 { break; }

        // 2. Create a Zero-Copy Packet
        let packet = Packet {
            data: PacketData::Physical {
                addr: frame_addr,
                len: chunk,
            }
        };

        // 3. Send via the output socket
        // Note: Real implementation would need a trait or specific socket check.
        // Here we simulate the network stack call.
        out_desc.handle.lock().write_zerocopy(packet)?;
        
        total_sent += chunk;
    }

    if !offset_ptr.is_null() {
        unsafe { *offset_ptr += total_sent as i64; }
    } else {
        in_desc.handle.lock().seek(crate::modules::vfs::SeekFrom::Current(total_sent as i64))?;
    }

    Ok(total_sent)
}

/// Extension trait for Zero-Copy Socket operations
pub trait SocketZeroCopy {
    fn write_zerocopy(&mut self, _packet: Packet) -> Result<(), &'static str> {
        Err("zerocopy not supported on this socket")
    }
}

impl<T: File + ?Sized> SocketZeroCopy for T {}
