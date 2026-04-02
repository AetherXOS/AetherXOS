use super::types::SwitchInfo;

pub(super) fn apply_arch_switch_state(
    cpu: &aethercore::kernel::cpu_local::CpuLocal,
    switch_info: &SwitchInfo,
) {
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    {
        use core::sync::atomic::Ordering;
        use aethercore::generated_consts::CORE_ENABLE_TLS_SYSCALLS;
        use aethercore::interfaces::cpu::CpuRegisters;

        cpu.kernel_stack_top
            .store(switch_info.next_kernel_sp, Ordering::Relaxed);
        if CORE_ENABLE_TLS_SYSCALLS {
            aethercore::hal::cpu::ArchCpuRegisters::write_tls_base(switch_info.next_tls);
        }

        let current_cr3 = aethercore::hal::cpu::ArchCpuRegisters::read_page_table_root();
        if current_cr3 != switch_info.next_cr3 {
            aethercore::hal::cpu::ArchCpuRegisters::write_page_table_root(switch_info.next_cr3);
        }
    }
}
