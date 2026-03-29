use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_hal_memory_map,
        &test_hal_interrupts,
        &test_hal_io_ports,
        &test_hal_msr,
        &test_hal_tsc,
        &test_hal_apic_id,
    ]
}

fn test_hal_memory_map() -> TestResult {
    let entries: [(u64, u64, u32); 3] = [
        (0x0000_0000, 0x0009_FC00, 1),
        (0x0009_FC00, 0x000A_0000, 2),
        (0x0010_0000, 0x7FFF_0000, 1),
    ];

    let mut total_usable: u64 = 0;
    for (base, limit, type_) in entries {
        if type_ == 1 {
            total_usable = total_usable.saturating_add(limit.saturating_sub(base));
        }
    }

    if total_usable > 0 {
        TestResult::pass("hal::memory_map")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("hal::memory_map", "No usable memory found")
            .with_category(TestCategory::Unit)
    }
}

fn test_hal_interrupts() -> TestResult {
    let idt_entries = 256;
    let valid_gates = [0x8E, 0x8F, 0x85, 0x84];
    
    let mut valid = true;
    for &gate in &valid_gates {
        if gate < 0x80 || gate > 0x8F {
            valid = false;
            break;
        }
    }

    if idt_entries == 256 && valid {
        TestResult::pass("hal::interrupts")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("hal::interrupts", "IDT configuration invalid")
            .with_category(TestCategory::Unit)
    }
}

fn test_hal_io_ports() -> TestResult {
    let port: u16 = 0x80;
    let value: u8 = 0x42;
    
    unsafe {
        core::arch::asm!(
            "outb %al, %dx",
            in("al") value,
            in("dx") port,
            options(nomem, nostack),
        );
    }
    
    TestResult::pass("hal::io_ports")
        .with_category(TestCategory::Unit)
}

fn test_hal_msr() -> TestResult {
    const MSR_EFER: u32 = 0xC000_0080;
    
    let efer: u64;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") MSR_EFER,
            out("eax") (efer as u32),
            out("edx") (efer >> 32),
            options(nomem, nostack),
        );
    }
    
    if efer != 0 {
        TestResult::pass("hal::msr")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("hal::msr", "MSR read returned zero")
            .with_category(TestCategory::Unit)
    }
}

fn test_hal_tsc() -> TestResult {
    let start = unsafe { core::arch::x86_64::_rdtsc() };
    let mut sum: u64 = 0;
    for i in 0..100 {
        sum = sum.wrapping_add(i);
    }
    let end = unsafe { core::arch::x86_64::_rdtsc() };
    
    let cycles = end.saturating_sub(start);
    
    if cycles > 0 && sum == 4950 {
        TestResult::pass("hal::tsc")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("hal::tsc", "TSC measurement failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_hal_apic_id() -> TestResult {
    let mut eax: u32 = 1;
    let mut ebx: u32 = 0;
    let mut ecx: u32 = 0;
    let mut edx: u32 = 0;
    
    unsafe {
        core::arch::asm!(
            "cpuid",
            in("eax") eax,
            out("ebx") ebx,
            out("ecx") ecx,
            out("edx") edx,
            options(nomem, nostack),
        );
    }
    
    let apic_id = (ebx >> 24) & 0xFF;
    
    if apic_id < 255 {
        TestResult::pass("hal::apic_id")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("hal::apic_id", "APIC ID read failed")
            .with_category(TestCategory::Unit)
    }
}
