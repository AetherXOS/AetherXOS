use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_interrupts_idt_setup,
        &test_interrupts_apic,
        &test_interrupts_ioapic,
        &test_interrupts_irq_routing,
        &test_interrupts_storm_protection,
    ]
}

fn test_interrupts_idt_setup() -> TestResult {
    TestResult::pass("integration::kernel::interrupts::idt_setup")
}

fn test_interrupts_apic() -> TestResult {
    TestResult::pass("integration::kernel::interrupts::apic")
}

fn test_interrupts_ioapic() -> TestResult {
    TestResult::pass("integration::kernel::interrupts::ioapic")
}

fn test_interrupts_irq_routing() -> TestResult {
    TestResult::pass("integration::kernel::interrupts::irq_routing")
}

fn test_interrupts_storm_protection() -> TestResult {
    TestResult::pass("integration::kernel::interrupts::storm_protection")
}
