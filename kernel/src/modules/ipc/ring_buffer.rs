use super::common::{current_task_id_or_kernel, suspend_on, wake_one_task, IPC_RING_BUFFER_SIZE_BYTES};
use crate::interfaces::IpcChannel;
use crate::kernel::sync::WaitQueue;
use core::cell::UnsafeCell;
use core::cmp;
use core::ptr;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

// Power of 2 buffer size for efficient masking
const BUFFER_SIZE: usize = IPC_RING_BUFFER_SIZE_BYTES;
const MASK: usize = BUFFER_SIZE - 1;
const FRAME_HEADER_BYTES: usize = 2;

#[derive(Debug, Clone, Copy)]
pub struct RingBufferStats {
    pub send_attempts: u64,
    pub send_enqueued: u64,
    pub send_dropped_full: u64,
    pub send_dropped_oversize: u64,
    pub receive_attempts: u64,
    pub receive_hits: u64,
    pub receive_empty: u64,
    pub receive_truncated: u64,
    pub protocol_incomplete_frame: u64,
    pub occupancy_overflow_guard: u64,
}

/// Professional Ring Buffer IPC.
///
/// Features:
/// - Single Producer, Single Consumer (SPSC) safe.
/// - Circular byte stream (Split messages supported).
/// - Format: [Length: u16 (LE)][Body...]
/// - Non-blocking (returns if full).
pub struct RingBuffer {
    buffer: UnsafeCell<[u8; BUFFER_SIZE]>,
    write_idx: AtomicUsize, // Monotonic
    read_idx: AtomicUsize,  // Monotonic
    producer: AtomicUsize,  // TaskId
    consumer: AtomicUsize,  // TaskId
    pub producer_wait: WaitQueue,
    pub consumer_wait: WaitQueue,
    send_attempts: AtomicU64,
    send_enqueued: AtomicU64,
    send_dropped_full: AtomicU64,
    send_dropped_oversize: AtomicU64,
    receive_attempts: AtomicU64,
    receive_hits: AtomicU64,
    receive_empty: AtomicU64,
    receive_truncated: AtomicU64,
    protocol_incomplete_frame: AtomicU64,
    occupancy_overflow_guard: AtomicU64,
}

// Safety: SPSC only.
unsafe impl Sync for RingBuffer {}

use alloc::sync::Arc;

pub struct RingBufferProducer {
    inner: Arc<RingBuffer>,
}

impl RingBufferProducer {
    pub fn send(&self, msg: &[u8]) {
        self.inner.send_internal(msg);
    }
}

impl IpcChannel for RingBufferProducer {
    fn send(&self, msg: &[u8]) {
        self.inner.send_internal(msg);
    }

    fn receive(&self, _buffer: &mut [u8]) -> Option<usize> {
        None
    }
}

pub struct RingBufferConsumer {
    pub(crate) inner: Arc<RingBuffer>,
}

impl RingBufferConsumer {
    pub fn receive(&self, buffer: &mut [u8]) -> Option<usize> {
        self.inner.receive_internal(buffer)
    }
}

impl IpcChannel for RingBufferConsumer {
    fn send(&self, _msg: &[u8]) {}

    fn receive(&self, buffer: &mut [u8]) -> Option<usize> {
        self.inner.receive_internal(buffer)
    }
}

impl RingBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: UnsafeCell::new([0; BUFFER_SIZE]),
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
            producer: AtomicUsize::new(0),
            consumer: AtomicUsize::new(0),
            producer_wait: WaitQueue::new(),
            consumer_wait: WaitQueue::new(),
            send_attempts: AtomicU64::new(0),
            send_enqueued: AtomicU64::new(0),
            send_dropped_full: AtomicU64::new(0),
            send_dropped_oversize: AtomicU64::new(0),
            receive_attempts: AtomicU64::new(0),
            receive_hits: AtomicU64::new(0),
            receive_empty: AtomicU64::new(0),
            receive_truncated: AtomicU64::new(0),
            protocol_incomplete_frame: AtomicU64::new(0),
            occupancy_overflow_guard: AtomicU64::new(0),
        }
    }

    pub fn split(self) -> (RingBufferProducer, RingBufferConsumer) {
        let arc = Arc::new(self);
        (
            RingBufferProducer { inner: arc.clone() },
            RingBufferConsumer { inner: arc },
        )
    }

    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            send_attempts: self.send_attempts.load(Ordering::Relaxed),
            send_enqueued: self.send_enqueued.load(Ordering::Relaxed),
            send_dropped_full: self.send_dropped_full.load(Ordering::Relaxed),
            send_dropped_oversize: self.send_dropped_oversize.load(Ordering::Relaxed),
            receive_attempts: self.receive_attempts.load(Ordering::Relaxed),
            receive_hits: self.receive_hits.load(Ordering::Relaxed),
            receive_empty: self.receive_empty.load(Ordering::Relaxed),
            receive_truncated: self.receive_truncated.load(Ordering::Relaxed),
            protocol_incomplete_frame: self.protocol_incomplete_frame.load(Ordering::Relaxed),
            occupancy_overflow_guard: self.occupancy_overflow_guard.load(Ordering::Relaxed),
        }
    }

    pub fn has_data(&self) -> bool {
        let read = self.read_idx.load(Ordering::Acquire);
        let write = self.write_idx.load(Ordering::Acquire);
        write != read
    }

    pub fn has_space_for(&self, payload_len: usize) -> bool {
        if payload_len > 0xFFFF {
            return false;
        }

        let needed = FRAME_HEADER_BYTES + payload_len;
        let write = self.write_idx.load(Ordering::Acquire);
        let read = self.read_idx.load(Ordering::Acquire);
        let used = write.saturating_sub(read);
        if used > BUFFER_SIZE {
            return false;
        }
        (BUFFER_SIZE - used) >= needed
    }

    #[cfg(test)]
    fn force_indices_for_test(&self, read: usize, write: usize) {
        self.read_idx.store(read, Ordering::Relaxed);
        self.write_idx.store(write, Ordering::Relaxed);
    }

    /// Internal helper to write bytes circularly
    unsafe fn write_circular(&self, start_idx: usize, data: &[u8]) {
        let buf_ptr = self.buffer.get() as *mut u8;
        let len = data.len();
        let offset = start_idx & MASK;

        // Calculate split
        let end = offset + len;
        if end <= BUFFER_SIZE {
            // Contiguous write
            // Safety: caller ensures capacity; source and destination do not overlap within the ring buffer.
            unsafe { ptr::copy_nonoverlapping(data.as_ptr(), buf_ptr.add(offset), len) };
        } else {
            // Split write
            let first_part = BUFFER_SIZE - offset;
            let second_part = len - first_part;

            // Safety: split copies target disjoint tail/head slices within the same backing ring buffer.
            unsafe { ptr::copy_nonoverlapping(data.as_ptr(), buf_ptr.add(offset), first_part) };
            // Safety: `first_part <= len`, so advancing the source pointer stays within `data`.
            unsafe {
                ptr::copy_nonoverlapping(data.as_ptr().add(first_part), buf_ptr, second_part)
            };
        }
    }

    /// Internal helper to read bytes circularly
    unsafe fn read_circular(&self, start_idx: usize, out_buf: &mut [u8]) {
        let buf_ptr = self.buffer.get() as *const u8;
        let len = out_buf.len();
        let offset = start_idx & MASK;

        let end = offset + len;
        if end <= BUFFER_SIZE {
            // Safety: caller provides an output buffer of `len`; source and destination are disjoint.
            unsafe { ptr::copy_nonoverlapping(buf_ptr.add(offset), out_buf.as_mut_ptr(), len) };
        } else {
            let first_part = BUFFER_SIZE - offset;
            let second_part = len - first_part;

            // Safety: split copies read disjoint tail/head segments from the ring buffer into `out_buf`.
            unsafe {
                ptr::copy_nonoverlapping(buf_ptr.add(offset), out_buf.as_mut_ptr(), first_part)
            };
            // Safety: `first_part <= len`, so advancing the output pointer stays within `out_buf`.
            unsafe {
                ptr::copy_nonoverlapping(buf_ptr, out_buf.as_mut_ptr().add(first_part), second_part)
            };
        }
    }
}

impl RingBuffer {
    pub(crate) fn send_internal(&self, msg: &[u8]) {
        self.send_attempts.fetch_add(1, Ordering::Relaxed);

        let tid = current_task_id_or_kernel().0;

        // Enforce producer affinity
        let mut producer = self.producer.load(Ordering::Acquire);
        if producer == 0 {
            if self
                .producer
                .compare_exchange(0, tid, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                producer = tid;
            } else {
                producer = self.producer.load(Ordering::Acquire);
            }
        }

        if producer != tid {
            return;
        }

        let len = msg.len();
        if len > 0xFFFF {
            self.send_dropped_oversize.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let total_bytes = FRAME_HEADER_BYTES + len;

        loop {
            // 1. Load indices
            let write = self.write_idx.load(Ordering::Relaxed);
            let read = self.read_idx.load(Ordering::Acquire);

            // 2. Check capacity
            let used = write.wrapping_sub(read);
            let free = BUFFER_SIZE - used;

            if free >= total_bytes {
                // 3. Write Length Header and Body
                let len_bytes = (len as u16).to_le_bytes();
                unsafe {
                    self.write_circular(write, &len_bytes);
                    self.write_circular(write + FRAME_HEADER_BYTES, msg);
                }

                // 4. Commit write
                self.write_idx.store(write + total_bytes, Ordering::Release);
                self.send_enqueued.fetch_add(1, Ordering::Relaxed);

                // 5. Wake consumer
                wake_one_task(&self.consumer_wait);
                return;
            }

            // Full: Block wait
            suspend_on(&self.producer_wait);
        }
    }

    pub(crate) fn receive_internal(&self, buffer: &mut [u8]) -> Option<usize> {
        self.receive_attempts.fetch_add(1, Ordering::Relaxed);

        let tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| cpu.current_task.load(Ordering::Relaxed))
                .unwrap_or(0)
        };

        // Enforce consumer affinity
        let mut consumer = self.consumer.load(Ordering::Acquire);
        if consumer == 0 {
            if self
                .consumer
                .compare_exchange(0, tid, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                consumer = tid;
            } else {
                consumer = self.consumer.load(Ordering::Acquire);
            }
        }

        if consumer != tid {
            return None;
        }

        loop {
            // 1. Load indices
            let read = self.read_idx.load(Ordering::Relaxed);
            let write = self.write_idx.load(Ordering::Acquire);

            if read != write {
                // 2. Read Length Header
                let mut len_bytes = [0u8; 2];
                unsafe {
                    self.read_circular(read, &mut len_bytes);
                }
                let msg_len = u16::from_le_bytes(len_bytes) as usize;
                let total_packet_size = FRAME_HEADER_BYTES + msg_len;

                // 3. Read Body
                let copy_len = cmp::min(msg_len, buffer.len());
                unsafe {
                    self.read_circular(read + FRAME_HEADER_BYTES, &mut buffer[0..copy_len]);
                }

                if copy_len < msg_len {
                    self.receive_truncated.fetch_add(1, Ordering::Relaxed);
                }

                // 4. Commit read
                self.read_idx
                    .store(read + total_packet_size, Ordering::Release);
                self.receive_hits.fetch_add(1, Ordering::Relaxed);

                // 5. Wake producer
                wake_one_task(&self.producer_wait);

                return Some(copy_len);
            }

            // Empty: Block wait
            suspend_on(&self.consumer_wait);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn ring_buffer_partial_consumer_drops_remaining_frame_bytes() {
        let rb = RingBuffer::new();
        let (producer, consumer) = rb.split();
        producer.send(b"abcdef");

        let mut out = [0u8; 3];
        let got = consumer.receive(&mut out);
        assert_eq!(got, Some(3));
        assert_eq!(&out, b"abc");

        let mut out2 = [0u8; 8];
        assert_eq!(consumer.receive(&mut out2), None);

        let stats = consumer.inner.stats();
        assert_eq!(stats.receive_truncated, 1);
    }

    #[test_case]
    fn ring_buffer_wrap_boundary_frame_roundtrip() {
        let rb = RingBuffer::new();
        rb.force_indices_for_test(BUFFER_SIZE - 1, BUFFER_SIZE - 1);
        let (producer, consumer) = rb.split();

        producer.send(b"hello");

        let mut out = [0u8; 8];
        let got = consumer.receive(&mut out);
        assert_eq!(got, Some(5));
        assert_eq!(&out[..5], b"hello");
    }

    #[test_case]
    fn ring_buffer_full_drop_is_accounted() {
        let rb = RingBuffer::new();
        let read = 1usize;
        let write = read + (BUFFER_SIZE - 3);
        rb.force_indices_for_test(read, write);
        let (producer, _) = rb.split();

        producer.send(b"xx");
        let stats = producer.inner.stats();
        assert_eq!(stats.send_dropped_full, 1);
        assert_eq!(stats.send_enqueued, 0);
    }
}
