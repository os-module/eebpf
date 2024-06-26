use core::ops::{Index, IndexMut};

use bit_field::BitField;
use kprobe::KprobeOps;
use log::info;
use polyhal::{TrapFrame, TrapFrameArgs};

use crate::kprobe::{PtRegs, DEBUG_KPROBE_LIST};

pub fn debug_handler(trap_context: &mut TrapFrame) {
    println!("<debug_handler>");
    let pc = *trap_context.index(TrapFrameArgs::SEPC);
    let kprobe = DEBUG_KPROBE_LIST.lock().get(&pc).map(|k| k.clone());
    if let Some(kprobe) = kprobe {
        kprobe.call_post_handler(&PtRegs::from(trap_context.clone()));
        let tf = trap_context.rflags.get_bit(8);
        info!("tf: {}", tf);
        info!("clear x86 single step");
        // clear single step
        trap_context.rflags.set_bit(8, false);
        // recover pc
        *trap_context.index_mut(TrapFrameArgs::SEPC) = kprobe.return_address();
    } else {
        info!("There is no kprobe in pc {:#x}", pc);
        // trap_context.rip += 1; // skip ebreak instruction
        panic!("skip ebreak instruction")
    }
}
