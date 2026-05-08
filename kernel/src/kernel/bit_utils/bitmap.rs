use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;

/// A thread-safe, high-performance bitmap for managing large sets of bits.
/// Ideal for page allocators, IRQ bitmaps, or process ID tracking.
pub struct BitMap {
    data: Vec<AtomicUsize>,
    bit_count: usize,
}

impl BitMap {
    const BITS_PER_WORD: usize = core::mem::size_of::<usize>() * 8;

    /// Create a new bitmap with the specified number of bits.
    pub fn new(count: usize) -> Self {
        let word_count = (count + Self::BITS_PER_WORD - 1) / Self::BITS_PER_WORD;
        let mut data = Vec::with_capacity(word_count);
        for _ in 0..word_count {
            data.push(AtomicUsize::new(0));
        }
        Self { data, bit_count: count }
    }

    /// Set a bit atomically. Returns the previous value.
    pub fn set(&self, bit: usize) -> bool {
        if bit >= self.bit_count { return false; }
        let word_idx = bit / Self::BITS_PER_WORD;
        let bit_idx = bit % Self::BITS_PER_WORD;
        let mask = 1 << bit_idx;
        (self.data[word_idx].fetch_or(mask, Ordering::SeqCst) & mask) != 0
    }

    /// Clear a bit atomically. Returns the previous value.
    pub fn clear(&self, bit: usize) -> bool {
        if bit >= self.bit_count { return false; }
        let word_idx = bit / Self::BITS_PER_WORD;
        let bit_idx = bit % Self::BITS_PER_WORD;
        let mask = 1 << bit_idx;
        (self.data[word_idx].fetch_and(!mask, Ordering::SeqCst) & mask) != 0
    }

    /// Test a bit.
    pub fn test(&self, bit: usize) -> bool {
        if bit >= self.bit_count { return false; }
        let word_idx = bit / Self::BITS_PER_WORD;
        let bit_idx = bit % Self::BITS_PER_WORD;
        (self.data[word_idx].load(Ordering::Relaxed) & (1 << bit_idx)) != 0
    }

    /// Find the first free bit (0) and set it to 1 atomically.
    /// Returns the index of the found bit, or None if the bitmap is full.
    pub fn find_and_set(&self) -> Option<usize> {
        for (word_idx, word) in self.data.iter().enumerate() {
            let mut val = word.load(Ordering::Relaxed);
            while val != !0usize {
                let free_bit = val.trailing_ones() as usize;
                let bit_idx = word_idx * Self::BITS_PER_WORD + free_bit;
                if bit_idx >= self.bit_count { break; }
                
                let mask = 1 << free_bit;
                match word.compare_exchange_weak(val, val | mask, Ordering::SeqCst, Ordering::Relaxed) {
                    Ok(_) => return Some(bit_idx),
                    Err(actual) => val = actual,
                }
            }
        }
        None
    }

    /// Count total set bits.
    pub fn count_set(&self) -> usize {
        self.data.iter().map(|w| w.load(Ordering::Relaxed).count_ones() as usize).sum()
    }

    pub fn size(&self) -> usize {
        self.bit_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_basic() {
        let bm = BitMap::new(100);
        assert!(!bm.test(42));
        assert!(!bm.set(42));
        assert!(bm.test(42));
        assert!(bm.clear(42));
        assert!(!bm.test(42));
    }

    #[test]
    fn test_bitmap_find_set() {
        let bm = BitMap::new(64);
        for i in 0..63 { bm.set(i); }
        assert_eq!(bm.find_and_set(), Some(63));
        assert_eq!(bm.find_and_set(), None);
    }
}
