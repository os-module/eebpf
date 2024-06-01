#![no_main]
#![no_std]
#![feature(naked_functions)]
#![feature(asm_const)]
extern crate alloc;

mod boot;
#[macro_use]
mod console;
mod arch;
mod kprobe;
mod sbi;
mod timer;
mod trap;

fn main() {
    trap::init_trap_subsystem();
    kprobe::test_kprobe();
    println!("OS shutdown!");
    sbi::system_shutdown();
}
