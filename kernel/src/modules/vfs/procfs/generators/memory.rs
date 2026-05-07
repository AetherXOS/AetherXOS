use super::super::*;

pub fn generate_meminfo() -> String {
    let free_pages = crate::modules::allocators::bitmap_pmm::get_free_pages();
    let total_pages = crate::modules::allocators::bitmap_pmm::PMM_TOTAL_PAGES;
    let total_kb = (total_pages as u64 * 4096) / 1024;
    let free_kb = (free_pages as u64 * 4096) / 1024;
    let available_kb = free_kb; // Simplification

    format!(
        "MemTotal:       {:8} kB\n\
         MemFree:        {:8} kB\n\
         MemAvailable:   {:8} kB\n\
         Buffers:               0 kB\n\
         Cached:                0 kB\n\
         SwapCached:            0 kB\n\
         Active:         {:8} kB\n\
         Inactive:              0 kB\n\
         SwapTotal:             0 kB\n\
         SwapFree:              0 kB\n\
         Dirty:                 0 kB\n\
         Writeback:             0 kB\n\
         AnonPages:             0 kB\n\
         Mapped:                0 kB\n\
         Shmem:                 0 kB\n\
         Slab:                  0 kB\n\
         SReclaimable:          0 kB\n\
         SUnreclaim:            0 kB\n\
         KernelStack:         256 kB\n\
         PageTables:          128 kB\n\
         CommitLimit:    {:8} kB\n\
         Committed_AS:          0 kB\n\
         VmallocTotal:   34359738367 kB\n\
         VmallocUsed:           0 kB\n\
         VmallocChunk:   34359737344 kB\n\
         HugePages_Total:       0\n\
         HugePages_Free:        0\n\
         HugePages_Rsvd:        0\n\
         HugePages_Surp:        0\n\
         Hugepagesize:       2048 kB\n",
        total_kb,
        free_kb,
        available_kb,
        total_kb - free_kb,
        total_kb,
    )
}
