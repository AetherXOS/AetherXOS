use super::types::SwitchInfo;

pub(super) fn apply_arch_switch_state(
    _cpu: &hypercore::kernel::cpu_local::CpuLocal,
    _switch_info: &SwitchInfo,
) {
    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    {
        use core::sync::atomic::Ordering;
        use hypercore::generated_consts::CORE_ENABLE_TLS_SYSCALLS;
        use hypercore::interfaces::cpu::CpuRegisters;

        _cpu.kernel_stack_top
            .store(_switch_info.next_kernel_sp, Ordering::Relaxed);
        if CORE_ENABLE_TLS_SYSCALLS {
            hypercore::hal::cpu::ArchCpuRegisters::write_tls_base(_switch_info.next_tls);
        }

        let current_cr3 = hypercore::hal::cpu::ArchCpuRegisters::read_page_table_root();
        if current_cr3 != _switch_info.next_cr3 {
            hypercore::hal::cpu::ArchCpuRegisters::write_page_table_root(_switch_info.next_cr3);
        }
    }
}
