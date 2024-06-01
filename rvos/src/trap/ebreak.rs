use crate::kprobe::KPROBE_LIST;
use crate::trap::context::TrapFrame;
use alloc::collections::BTreeMap;
use spin::Mutex;

static KPROBE_RUN_LIST: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

pub fn ebreak_handler(trap_context: &mut TrapFrame) {
    let pc = trap_context.sepc;
    let mut kporbe = KPROBE_LIST.lock();
    let kprobe = kporbe.get_mut(&pc);
    if let Some(kprobe) = kprobe {
        kprobe.pre_handler(&trap_context.pt_regs);
        let (simulate_pc, simulate_ebreak) = kprobe.simulate_single_step();
        // simulate single step
        println!("simulate_single_step, set sepc: {:#x}", simulate_pc);
        trap_context.sepc = simulate_pc;
        KPROBE_RUN_LIST.lock().insert(simulate_ebreak, pc);
    } else {
        let mut run_list = KPROBE_RUN_LIST.lock();
        let run = run_list.get(&pc);
        if let Some(run) = run {
            println!("The kprobe which pc {:#x} is in run list", run);
            let kprobe = kporbe.get_mut(run).unwrap();
            kprobe.post_handler(&trap_context.pt_regs);
            let next_inst = kprobe.uninstall();
            trap_context.sepc = next_inst;
            run_list.remove(&pc);
        } else {
            println!("Ther is no kprobe in pc {:#x}", pc);
            trap_context.sepc += 4; // skip ebreak instruction
        }
    }
}
