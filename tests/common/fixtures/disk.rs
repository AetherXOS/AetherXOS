pub const TEST_DISK_SIZE_BYTES: usize = 64 * 1024 * 1024;
pub const TEST_DISK_SECTOR_SIZE: usize = 512;
pub const TEST_DISK_SECTOR_COUNT: usize = TEST_DISK_SIZE_BYTES / TEST_DISK_SECTOR_SIZE;

pub struct DiskFixture {
    pub data: Vec<u8>,
    pub sector_size: usize,
}

impl DiskFixture {
    pub fn new() -> Self {
        Self {
            data: vec![0u8; TEST_DISK_SIZE_BYTES],
            sector_size: TEST_DISK_SECTOR_SIZE,
        }
    }

    pub fn with_size(size_bytes: usize) -> Self {
        Self {
            data: vec![0u8; size_bytes],
            sector_size: TEST_DISK_SECTOR_SIZE,
        }
    }

    pub fn read_sector(&self, sector: usize) -> Option<&[u8]> {
        let start = sector.checked_mul(self.sector_size)?;
        let end = start.checked_add(self.sector_size)?;
        self.data.get(start..end)
    }

    pub fn write_sector(&mut self, sector: usize, data: &[u8]) -> Result<(), &'static str> {
        if data.len() != self.sector_size {
            return Err("Invalid sector data size");
        }
        let start = sector.checked_mul(self.sector_size).ok_or("Overflow")?;
        let end = start.checked_add(self.sector_size).ok_or("Overflow")?;
        
        if end > self.data.len() {
            return Err("Sector out of bounds");
        }
        
        self.data[start..end].copy_from_slice(data);
        Ok(())
    }

    pub fn fill_pattern(&mut self, pattern: u8) {
        for byte in &mut self.data {
            *byte = pattern;
        }
    }

    pub fn fill_random(&mut self, seed: u64) {
        let mut state = seed;
        for byte in &mut self.data {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            *byte = (state >> 33) as u8;
        }
    }

    pub fn get_hash(&self) -> u64 {
        let mut hash: u64 = 0;
        for (i, &byte) in self.data.iter().enumerate() {
            hash = hash.wrapping_add((byte as u64).wrapping_mul(i as u64));
        }
        hash
    }
}

impl Default for DiskFixture {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_test_mbr() -> [u8; 512] {
    let mut mbr = [0u8; 512];
    mbr[510] = 0x55;
    mbr[511] = 0xAA;
    mbr
}

pub fn create_test_gpt_header() -> Vec<u8> {
    let mut header = vec![0u8; 512];
    header[0..8].copy_from_slice(b"EFI PART");
    header[8..12].copy_from_slice(&0x00010000u32.to_le_bytes());
    header[16..24].copy_from_slice(&0u64.to_le_bytes());
    header[24..32].copy_from_slice(&1u64.to_le_bytes());
    header
}
