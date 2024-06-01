pub mod context;
mod ebreak;

use crate::trap::context::TrapFrame;
use crate::{arch, println, timer};
use core::arch::global_asm;
use log::trace;
use riscv::register::{
    scause::{Exception, Interrupt, Trap},
    sepc, sscratch, sstatus,
    sstatus::SPP,
    stval, stvec,
    stvec::TrapMode,
};

global_asm!(include_str!("./kernel_v.asm"));

extern "C" {
    fn kernel_v();
}

/// 开启中断/异常
pub fn init_trap_subsystem() {
    unsafe {
        sstatus::set_spp(SPP::Supervisor);
    }
    println!("++++ setup interrupt ++++");
    set_kernel_trap_entry();
    arch::external_interrupt_enable();
    arch::timer_interrupt_enable();
    arch::interrupt_enable();
    let enable = arch::is_interrupt_enable();
    println!("++++ setup interrupt done, enable:{:?} ++++", enable);
}
/// 设置内核态 trap 处理例程的入口点
#[inline]
pub fn set_kernel_trap_entry() {
    unsafe {
        sscratch::write(kernel_trap_vector as usize);
        stvec::write(kernel_v as usize, TrapMode::Direct);
    }
}

/// 只有在内核态下才能进入这个函数
/// 避免嵌套中断发生这里不会再开启中断
#[no_mangle]
pub fn kernel_trap_vector(trap_context: &mut TrapFrame) {
    let sstatus = trap_context.sstatus;
    let spp = sstatus.spp();
    if spp == SPP::User {
        panic!("kernel_trap_vector: spp == SPP::User");
    }
    let enable = arch::is_interrupt_enable();
    assert!(!enable);
    let cause = riscv::register::scause::read().cause();
    println!("kernel_trap_vector: cause: {:?}", cause);
    cause.do_kernel_handle(trap_context)
}

pub trait TrapHandler {
    fn do_kernel_handle(&self, trap_context: &mut TrapFrame);
}

impl TrapHandler for Trap {
    fn do_kernel_handle(&self, trap_context: &mut TrapFrame) {
        let stval = stval::read();
        let sepc = sepc::read();
        match self {
            Trap::Interrupt(Interrupt::SupervisorTimer) => {
                trace!("[kernel] timer interrupt");
                timer::set_next_trigger()
            }
            Trap::Exception(Exception::Breakpoint) => ebreak::ebreak_handler(trap_context),
            Trap::Interrupt(Interrupt::SupervisorExternal) => {
                unimplemented!()
            }
            _ => {
                panic!(
                    "unhandled trap: {:?}, stval: {:?}, sepc: {:x}",
                    self, stval, sepc
                )
            }
        }
    }
}
