use core::{arch::asm, sync::atomic::AtomicBool};

use jtable::*;
use paste::*;

#[naked_function::naked]
pub unsafe extern "C" fn is_false() -> bool {
    asm!("li a0, 1", "ret",)
}

const FALSE: u16 = 0b_010_0_01010_00000_01;
const TRUE: u16 = 0b_010_0_01010_00001_01;

const LOOP: usize = 10_0000;

static FLAG: AtomicBool = AtomicBool::new(true);

#[inline(never)]
#[no_mangle]
fn load_flag() -> bool {
    let inst_now = riscv::register::instret::read();
    let res = FLAG.load(core::sync::atomic::Ordering::SeqCst);
    let inst_end = riscv::register::instret::read();
    println!("load_flag: {}inst", inst_end - inst_now);
    res
}

#[inline(never)]
#[no_mangle]
unsafe fn load_fun() -> bool {
    let inst_now = riscv::register::instret::read();
    let res = is_false();
    let inst_end = riscv::register::instret::read();
    println!("load_fun: {}inst", inst_end - inst_now);
    res
}

#[inline(never)]
#[no_mangle]
fn load_nop() -> bool {
    let inst_now = riscv::register::instret::read();
    unsafe { asm!("nop") }
    let inst_end = riscv::register::instret::read();
    println!("load_nop: {}inst", inst_end - inst_now);
    true
}

pub unsafe fn test() {
    load_fun();
    load_flag();
    load_nop();

    let mut count = 0;
    let now = polyhal::time::Time::now().to_usec();
    for _ in 0..LOOP {
        maybe_modify();
        if FLAG.load(core::sync::atomic::Ordering::SeqCst) {
            count += 1;
        }
    }
    let end = polyhal::time::Time::now().to_usec();
    println!("test_atomic: {}us", end - now);
    println!("test_atomic: {}", count);

    let mut count = 0;
    let now = polyhal::time::Time::now().to_usec();
    for _ in 0..LOOP {
        maybe_modify();
        if is_false() {
            count += 1;
        }
    }
    let end = polyhal::time::Time::now().to_usec();
    println!("test_static_keys: {}us", end - now);
    println!("test_static_keys: {}", count);
    println!("is_false: {:#x}", is_false as usize);

    test_static_key();
}

fn maybe_modify() {
    let time = polyhal::time::Time::now().raw();
    if time < 100 {
        FLAG.store(false, core::sync::atomic::Ordering::SeqCst);
    }
}

define_static_key_true!(TRUE_MASK);
define_static_key_false!(FALSE_MASK);

#[no_mangle]
#[inline(never)]
pub fn test_static_key() {
    if static_branch_likely!(TRUE_MASK) {
        println!("static_branch_likely");
    }

    static_branch_disable!(TRUE_MASK);

    if static_branch_likely!(TRUE_MASK) {
        println!("static_branch_likely XXXX");
    } else {
        println!("static_branch_likely FFFF");
    }
    if static_branch_unlikely!(FALSE_MASK) {
        println!("static_branch_unlikely");
    }

    static_branch_enable!(FALSE_MASK);
    if static_branch_unlikely!(FALSE_MASK) {
        println!("static_branch_unlikely XXXX");
    } else {
        println!("static_branch_unlikely FFFF");
    }
}
