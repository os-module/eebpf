use core::ops::{Index, IndexMut};

use kprobe::KprobeOps;
use log::info;
use polyhal::{TrapFrame, TrapFrameArgs};

use crate::kprobe::{PtRegs, BREAK_KPROBE_LIST};

pub fn ebreak_handler(trap_context: &mut TrapFrame) {
    let break_addr = if cfg!(target_arch = "x86_64") {
        *trap_context.index(TrapFrameArgs::SEPC) - 1
    } else if cfg!(target_arch = "riscv64") {
        *trap_context.index(TrapFrameArgs::SEPC) - 2
    } else if cfg!(target_arch = "loongarch64") {
        *trap_context.index(TrapFrameArgs::SEPC) - 4
    } else {
        panic!("unsupported arch")
    };

    println!("<ebreak_handler>: break_addr: {:#x}", break_addr);

    let kprobe = BREAK_KPROBE_LIST.lock().get(&break_addr).map(|k| k.clone());
    if let Some(kprobe) = kprobe {
        kprobe.call_pre_handler(&PtRegs::from(trap_context.clone()));
        // set single step
        #[cfg(target_arch = "x86_64")]
        {
            info!("set x86 single step");
            trap_context.rflags |= 0x100;
        }
        let step_addr = kprobe.single_step_address();
        info!("old_instruction address: {:#x}", step_addr);
        *trap_context.index_mut(TrapFrameArgs::SEPC) = step_addr;
    } else {
        #[cfg(not(target_arch = "x86_64"))]
        {
            let run_list = crate::kprobe::DEBUG_KPROBE_LIST.lock();
            let run = run_list.get(&break_addr);
            if let Some(kprobe) = run {
                println!("The kprobe which pc {:#x} is in run list", break_addr);
                kprobe.call_post_handler(&PtRegs::from(trap_context.clone()));
                let next_inst = kprobe.return_address();
                info!("set sepc: {:#x}", next_inst);
                *trap_context.index_mut(TrapFrameArgs::SEPC) = next_inst;
            } else {
                println!("Ther is no kprobe in pc {:#x}", break_addr);
            }
        }
        #[cfg(target_arch = "x86_64")]
        println!("Ther is no kprobe in pc {:#x}", break_addr);
    }
}
