#[cfg(target_arch = "riscv64")]
mod riscv;
#[cfg(target_arch = "x86-64")]
mod x86;

#[cfg(target_arch = "riscv64")]
pub use riscv::*;
#[cfg(target_arch = "x86-64")]
pub use x86::*;

#[macro_export]
macro_rules! gen_mask {
    ($high:expr, $low:expr) => {
        ((1 << ($high - $low + 1)) - 1) << $low
    };
}

#[cfg(test)]
mod test {
    #[test]
    fn test_gen_mask() {
        assert_eq!(gen_mask!(3, 0), 0b1111);
        assert_eq!(gen_mask!(7, 4), 0b11110000);
    }
}
