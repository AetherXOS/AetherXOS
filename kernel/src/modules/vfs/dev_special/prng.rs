use core::sync::atomic::{AtomicU64, Ordering};

/// Global PRNG state seeded from RDRAND or TSC at boot.
pub(crate) static PRNG_STATE: AtomicU64 = AtomicU64::new(0x6A09E667F3BCC908);

/// Mix function based on SplitMix64 — fast, decent quality for /dev/urandom.
#[inline(always)]
pub(crate) fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Seed the PRNG from hardware entropy (call during init).
pub fn seed_prng(entropy: u64) {
    PRNG_STATE.store(entropy, Ordering::Relaxed);
}

/// Fill a buffer with pseudo-random bytes.
pub(crate) fn fill_random_bytes(buf: &mut [u8]) {
    let mut state = PRNG_STATE.load(Ordering::Relaxed);
    state ^= crate::hal::cpu::rdtsc();

    let mut pos = 0;
    while pos < buf.len() {
        let word = splitmix64(&mut state);
        let bytes = word.to_le_bytes();
        let remaining = buf.len() - pos;
        let copy_len = remaining.min(8);
        buf[pos..pos + copy_len].copy_from_slice(&bytes[..copy_len]);
        pos += copy_len;
    }
    PRNG_STATE.store(state, Ordering::Relaxed);
}
