pub trait Bits {
    fn popcount(self) -> u32;
    fn leading_zeros(self) -> u32;
    fn trailing_zeros(self) -> u32;
    fn reverse_bits(self) -> Self;
    fn is_power_of_two(self) -> bool;
    fn align_up(self, align: Self) -> Self;
    fn align_down(self, align: Self) -> Self;
}

macro_rules! impl_bits {
    ($($t:ty),*) => {
        $(
            impl Bits for $t {
                #[inline(always)] fn popcount(self) -> u32 { self.count_ones() }
                #[inline(always)] fn leading_zeros(self) -> u32 { self.leading_zeros() }
                #[inline(always)] fn trailing_zeros(self) -> u32 { self.trailing_zeros() }
                #[inline(always)] fn reverse_bits(self) -> Self { self.reverse_bits() }
                #[inline(always)] fn is_power_of_two(self) -> bool { self > 0 && (self & (self - 1)) == 0 }
                #[inline(always)] fn align_up(self, align: Self) -> Self {
                    let mask = align - 1;
                    (self + mask) & !mask
                }
                #[inline(always)] fn align_down(self, align: Self) -> Self {
                    let mask = align - 1;
                    self & !mask
                }
            }
        )*
    };
}

impl_bits!(u8, u16, u32, u64, usize);
