pub trait CpuRegisters {
    fn read_page_fault_addr() -> u64;
    fn read_page_table_root() -> u64;
    fn write_page_table_root(addr: u64);
    fn read_tls_base() -> u64;
    fn write_tls_base(addr: u64);
    fn read_per_cpu_base() -> u64;
    fn write_per_cpu_base(addr: u64);
}
