use crate::arch;

pub const CLOCK_FREQ: usize = 1250_0000;
#[inline]
pub fn read_timer() -> usize {
    arch::read_timer()
}

#[inline]
pub fn set_next_trigger() {
    const TICKS_PER_SEC: usize = 10;
    let next = read_timer() + CLOCK_FREQ / TICKS_PER_SEC;
    assert!(next > read_timer());
    crate::sbi::set_timer(next);
}
