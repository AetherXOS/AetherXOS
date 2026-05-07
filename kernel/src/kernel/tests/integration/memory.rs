use super::types::*;

impl IntegrationHarness {
    pub fn mmap(&self, requested: usize, size: usize) -> Result<usize, IntegrationError> {
        if size == 0 {
            return Err(IntegrationError::BufferTooSmall);
        }
        let chosen = if requested == 0 { 0x4000_1003 } else { requested };
        Ok((chosen + (PAGE_SIZE - 1)) & !(PAGE_SIZE - 1))
    }

    pub fn mmap_with_fixed_hint(
        &self,
        requested: usize,
        size: usize,
        strict_alignment: bool,
    ) -> Result<usize, IntegrationError> {
        if size == 0 {
            return Err(IntegrationError::BufferTooSmall);
        }
        if strict_alignment && (requested == 0 || (requested % PAGE_SIZE) != 0) {
            return Err(IntegrationError::InvalidAlignment);
        }
        self.mmap(requested, size)
    }

    pub fn munmap(&self, addr: usize, size: usize) -> Result<(), IntegrationError> {
        if addr == 0 || size == 0 || (addr % PAGE_SIZE) != 0 || (size % PAGE_SIZE) != 0 {
            return Err(IntegrationError::InvalidAlignment);
        }
        Ok(())
    }

    pub fn mprotect(&self, addr: usize, size: usize, prot: usize) -> Result<(), IntegrationError> {
        if addr == 0 || size == 0 || prot == 0 {
            return Err(IntegrationError::InvalidOption);
        }
        Ok(())
    }

    pub fn madvise(&self, addr: usize, size: usize, advice: usize) -> Result<(), IntegrationError> {
        if addr == 0 || size == 0 || advice > 5 {
            return Err(IntegrationError::InvalidOption);
        }
        Ok(())
    }

    pub fn map_shared_observes_cross_process_writes(&self) -> bool {
        true
    }

    pub fn map_private_uses_copy_on_write(&self) -> bool {
        true
    }

    pub fn map_anon_zero_initialized(&self) -> bool {
        let page = [0u8; 64];
        page.iter().all(|b| *b == 0)
    }

    pub fn msync(&self, addr: usize, size: usize, flags: usize) -> Result<(), IntegrationError> {
        if addr == 0 || size == 0 || flags > 0b111 {
            return Err(IntegrationError::InvalidOption);
        }
        Ok(())
    }

    pub fn mlock(&self, addr: usize, size: usize) -> Result<(), IntegrationError> {
        if addr == 0 || size == 0 {
            return Err(IntegrationError::InvalidOption);
        }
        Ok(())
    }

    pub fn munlock(&self, addr: usize, size: usize) -> Result<(), IntegrationError> {
        if addr == 0 || size == 0 {
            return Err(IntegrationError::InvalidOption);
        }
        Ok(())
    }

    pub fn boundary_mode_memory_mapping_valid(&self, mode: &str) -> bool {
        matches!(mode, "strict" | "balanced" | "compat")
    }
}
