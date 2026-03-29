use core::sync::atomic::{AtomicU32, Ordering};

static ID_COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn unique_id() -> u32 {
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn generate_random_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(len);
    let mut state = seed;
    
    for _ in 0..len {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        result.push((state >> 33) as u8);
    }
    
    result
}

pub fn generate_sequential_bytes(start: u8, len: usize) -> Vec<u8> {
    (start..start.wrapping_add(len as u8)).collect()
}

pub fn generate_pattern_bytes(pattern: &[u8], repetitions: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(pattern.len() * repetitions);
    for _ in 0..repetitions {
        result.extend_from_slice(pattern);
    }
    result
}

pub fn generate_aligned_address(align: usize) -> usize {
    let ptr = 0x1000usize;
    (ptr + align - 1) & !(align - 1)
}

pub fn generate_test_string(len: usize) -> alloc::string::String {
    use alloc::string::String;
    let mut s = String::with_capacity(len);
    for i in 0..len {
        s.push((b'a' + (i % 26) as u8) as char);
    }
    s
}
