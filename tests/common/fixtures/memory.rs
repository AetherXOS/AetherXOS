pub const TEST_MEMORY_SIZE: usize = 16 * 1024 * 1024;
pub const TEST_PAGE_SIZE: usize = 4096;
pub const TEST_ALIGNMENT: usize = 4096;

pub struct MemoryFixture {
    pub data: Vec<u8>,
    pub size: usize,
}

impl MemoryFixture {
    pub fn new() -> Self {
        Self {
            data: vec![0u8; TEST_MEMORY_SIZE],
            size: TEST_MEMORY_SIZE,
        }
    }

    pub fn with_size(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
            size,
        }
    }

    pub fn fill(&mut self, value: u8) {
        for byte in &mut self.data {
            *byte = value;
        }
    }

    pub fn fill_pattern(&mut self, pattern: &[u8]) {
        for (i, byte) in self.data.iter_mut().enumerate() {
            *byte = pattern[i % pattern.len()];
        }
    }

    pub fn write_at(&mut self, offset: usize, data: &[u8]) -> Result<(), &'static str> {
        let end = offset.checked_add(data.len()).ok_or("Overflow")?;
        if end > self.data.len() {
            return Err("Out of bounds");
        }
        self.data[offset..end].copy_from_slice(data);
        Ok(())
    }

    pub fn read_at(&self, offset: usize, len: usize) -> Option<&[u8]> {
        let end = offset.checked_add(len)?;
        self.data.get(offset..end)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl Default for MemoryFixture {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_zeroed_page() -> [u8; TEST_PAGE_SIZE] {
    [0u8; TEST_PAGE_SIZE]
}

pub fn create_pattern_page(pattern: u8) -> [u8; TEST_PAGE_SIZE] {
    [pattern; TEST_PAGE_SIZE]
}

pub fn create_random_page(seed: u64) -> [u8; TEST_PAGE_SIZE] {
    let mut page = [0u8; TEST_PAGE_SIZE];
    let mut state = seed;
    for byte in &mut page {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *byte = (state >> 33) as u8;
    }
    page
}

pub fn create_guardian_page() -> [u8; TEST_PAGE_SIZE] {
    let mut page = [0u8; TEST_PAGE_SIZE];
    page[0..8].copy_from_slice(&0xDEADBEEFCAFEBABEu64.to_le_bytes());
    page[TEST_PAGE_SIZE - 8..].copy_from_slice(&0xDEADBEEFCAFEBABEu64.to_le_bytes());
    page
}
