#![no_std]
#![no_main]
#![feature(panic_info_message)]
extern crate alloc;

mod allocator;
mod frame;
#[macro_use]
mod logging;
mod ebreak;
mod kprobe;

#[cfg(target_arch = "x86_64")]
mod debug;
mod static_keys;


use core::panic::PanicInfo;

use frame::frame_alloc;
use polyhal::{
    addr::PhysPage, get_mem_areas, shutdown, PageAlloc, TrapFrame, TrapType, TrapType::*,
};

pub struct PageAllocImpl;

impl PageAlloc for PageAllocImpl {
    fn alloc(&self) -> PhysPage {
        frame_alloc()
    }

    fn dealloc(&self, ppn: PhysPage) {
        frame::frame_dealloc(ppn)
    }
}

/// kernel interrupt
#[polyhal::arch_interrupt]
fn kernel_interrupt(ctx: &mut TrapFrame, trap_type: TrapType) {
    // println!("trap_type @ {:x?} {:#x?}", trap_type, ctx);
    match trap_type {
        Breakpoint => {
            ebreak::ebreak_handler(ctx);
        }
        UserEnvCall => {
            // jump to next instruction anyway
            ctx.syscall_ok();
            log::info!("Handle a syscall");
        }
        StorePageFault(_paddr) | LoadPageFault(_paddr) | InstructionPageFault(_paddr) => {
            log::info!("page fault");
            panic!("page fault at {:#x?}", _paddr);
        }
        Debug => {
            #[cfg(target_arch = "x86_64")]
            debug::debug_handler(ctx);
        }
        IllegalInstruction(_) => {
            log::info!("illegal instruction");
        }
        Time => {
            log::info!("Timer");
        }
        _ => {
            log::warn!("unsuspended trap type: {:?}", trap_type);
        }
    }
}

#[polyhal::arch_entry]
/// kernel main function, entry point.
fn main(hartid: usize) {
    if hartid != 0 {
        return;
    }

    println!("[kernel] Hello, world!");
    allocator::init_allocator();
    logging::init(Some("trace"));
    println!("init logging");

    // Init page alloc for polyhal
    polyhal::init(&PageAllocImpl);

    get_mem_areas().into_iter().for_each(|(start, size)| {
        println!("init memory region {:#x} - {:#x}", start, start + size);
        frame::add_frame_range(start, start + size);
    });
    kprobe::test_kprobe();

    panic!("end of rust_main!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        log::error!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        log::error!("[kernel] Panicked: {}", info.message().unwrap());
    }
    shutdown()
}
