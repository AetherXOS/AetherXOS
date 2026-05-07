use super::super::*;

pub fn generate_cpuinfo() -> String {
    let cpu_count = crate::hal::smp::cpu_count().max(1);

    let mut result = String::new();
    for i in 0..cpu_count {
        result.push_str(&format!(
            "processor\t: {}\n\
             vendor_id\t: GenuineIntel\n\
             cpu family\t: 6\n\
             model\t\t: 158\n\
             model name\t: AetherCore Virtual CPU\n\
             stepping\t: 10\n\
             cpu MHz\t\t: 2400.000\n\
             cache size\t: 8192 KB\n\
             physical id\t: 0\n\
             siblings\t: {}\n\
             core id\t\t: {}\n\
             cpu cores\t: {}\n\
             apicid\t\t: {}\n\
             fpu\t\t: yes\n\
             fpu_exception\t: yes\n\
             cpuid level\t: 22\n\
             wp\t\t: yes\n\
             flags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss syscall nx pdpe1gb rdtscp lm constant_tsc rep_good nopl xtopology cpuid pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm abm cpuid_fault pti fsgsbase bmi1 avx2 bmi2 erms rdseed adx clflushopt\n\
             bogomips\t: 4800.00\n\
             clflush size\t: 64\n\
             cache_alignment\t: 64\n\
             address sizes\t: 48 bits physical, 48 bits virtual\n\
             power management:\n\n",
            i,
            cpu_count,
            i,
            cpu_count,
            i,
        ));
    }
    result
}
