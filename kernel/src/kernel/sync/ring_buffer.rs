use core::sync::atomic::{AtomicUsize, Ordering};
use core::cell::UnsafeCell;
use alloc::vec::Vec;

/// Generic Lock-Free Ring Buffer.
/// Used to eliminate code duplication across TTY, IO_URING, and Networking.
pub struct RingBuffer<T> {
    buffer: Vec<UnsafeCell<T>>,
    head: AtomicUsize,
    tail: AtomicUsize,
    capacity: usize,
}

impl<T: Default + Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(T::default()));
        }
        Self {
            buffer,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            capacity,
        }
    }

    pub fn push(&self, item: T) -> Result<(), &'static str> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail.wrapping_sub(head) >= self.capacity {
            return Err("buffer full");
        }

        unsafe {
            let ptr = self.buffer[tail % self.capacity].get();
            core::ptr::write(ptr, item);
        }

        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let item = unsafe {
            let ptr = self.buffer[head % self.capacity].get();
            // We clone here because we can't move out of UnsafeCell easily without core::ptr::read
            // and we need to keep a valid value there if T is not Copy.
            // Since T: Default + Clone, we can read and then potentially replace with Default if needed,
            // but for a ring buffer, we usually just leave it there until overwritten.
            (*ptr).clone()
        };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(item)
    }

    pub fn push_slice(&self, data: &[u8]) -> usize
    where
        T: From<u8> + Copy,
    {
        let mut written = 0;
        for &b in data {
            if self.push(T::from(b)).is_err() {
                break;
            }
            written += 1;
        }
        written
    }

    pub fn pop_slice(&self, data: &mut [u8]) -> usize
    where
        T: Into<u8> + Copy,
    {
        let mut read = 0;
        for b in data {
            if let Some(item) = self.pop() {
                *b = item.into();
                read += 1;
            } else {
                break;
            }
        }
        read
    }

    pub fn has_data(&self) -> bool {
        self.len() > 0
    }

    pub fn has_space_for(&self, count: usize) -> bool {
        self.capacity - self.len() >= count
    }

    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }
}
