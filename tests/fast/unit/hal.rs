use hypercore::hal::common::cpu_features::{
    field_at_least_u64, field_present_u64, field_u64, has_bit_u32,
};
use hypercore::hal::common::timer::{clamp_ticks, ns_to_ticks, ticks_to_ns};

#[test]
fn cpu_feature_helpers_decode_expected_bitfields() {
    assert!(has_bit_u32(0b1010, 1));
    assert!(has_bit_u32(0b1010, 3));
    assert!(!has_bit_u32(0b1010, 0));
    assert_eq!(field_u64(0xABCD, 8, 0xFF), 0xAB);
    assert!(field_present_u64(0x2F0, 4, 0xF, 0));
    assert!(field_at_least_u64(0x2F0, 4, 0xF, 0xF));
    assert!(!field_at_least_u64(0x020, 4, 0xF, 3));
}

#[test]
fn timer_helpers_convert_and_clamp_without_underflow() {
    assert_eq!(ns_to_ticks(10_000_000, 1_000_000, 1_000), 10_000);
    assert_eq!(ns_to_ticks(10_000_000, 0, 1_000), 10_000);
    assert_eq!(ns_to_ticks(10_000_000, 0, 0), 0);
    assert_eq!(ticks_to_ns(10_000, 1_000_000), 10_000_000);
    assert_eq!(ticks_to_ns(10_000, 0), 0);
    assert_eq!(clamp_ticks(5, 10, 100), (10, true, false));
    assert_eq!(clamp_ticks(500, 10, 100), (100, false, true));
    assert_eq!(clamp_ticks(50, 10, 100), (50, false, false));
}
