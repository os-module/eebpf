use alloc::{collections::BTreeMap, string::ToString, sync::Arc};

use kprobe::{Kprobe, KprobeBuilder, KprobeOps, ProbeArgs};
use polyhal::{hart_id, TrapFrame};
use spin::Mutex;

pub static BREAK_KPROBE_LIST: Mutex<BTreeMap<usize, Arc<Kprobe>>> = Mutex::new(BTreeMap::new());
pub static DEBUG_KPROBE_LIST: Mutex<BTreeMap<usize, Arc<Kprobe>>> = Mutex::new(BTreeMap::new());

#[cfg(target_arch = "x86_64")]
pub struct PtRegs {
    pub rax: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rbx: usize,
    pub rbp: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
}

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
pub struct PtRegs {
    pub x: [usize; 32],
}

impl ProbeArgs for PtRegs {
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}
#[cfg(target_arch = "x86_64")]
impl From<TrapFrame> for PtRegs {
    fn from(tf: TrapFrame) -> Self {
        Self {
            rax: tf.rax,
            rcx: tf.rcx,
            rdx: tf.rdx,
            rbx: tf.rbx,
            rbp: tf.rbp,
            rsi: tf.rsi,
            rdi: tf.rdi,
            r8: tf.r8,
            r9: tf.r9,
            r10: tf.r10,
            r11: tf.r11,
            r12: tf.r12,
            r13: tf.r13,
            r14: tf.r14,
            r15: tf.r15,
        }
    }
}

#[cfg(target_arch = "riscv64")]
impl From<TrapFrame> for PtRegs {
    fn from(tf: TrapFrame) -> Self {
        Self { x: tf.x }
    }
}

#[cfg(target_arch = "loongarch64")]
impl From<TrapFrame> for PtRegs {
    fn from(tf: TrapFrame) -> Self {
        Self { x: tf.regs }
    }
}

#[inline(never)]
#[no_mangle]
pub fn detect_func(x: usize, y: usize) -> usize {
    let hart = hart_id();
    println!("detect_func: hart_id: {}, x: {}, y:{}", hart, x, y);
    hart
}

pub fn test_kprobe() {
    let pre_handler = |regs: &dyn ProbeArgs| {
        let pt_regs = regs.as_any().downcast_ref::<PtRegs>().unwrap();
        println!("call pre_handler, the sp is {:#x}", 0);
    };
    let post_handler = |regs: &dyn ProbeArgs| {
        let pt_regs = regs.as_any().downcast_ref::<PtRegs>().unwrap();
        println!("call post_handler, the sp is {:#x}", 0);
    };
    let fault_handler = |regs: &dyn ProbeArgs| {
        let pt_regs = regs.as_any().downcast_ref::<PtRegs>().unwrap();
        println!("call fault_handler, the sp is {:#x}", 0);
    };

    let kprobe = KprobeBuilder::new()
        .symbol("detect_func".to_string())
        .symbol_addr(detect_func as usize)
        .offset(0)
        .pre_handler(pre_handler)
        .post_handler(post_handler)
        .fault_handler(fault_handler)
        .build()
        .install();

    let kprobe = Arc::new(kprobe);
    BREAK_KPROBE_LIST
        .lock()
        .insert(detect_func as usize, kprobe.clone());
    let debug_address = kprobe.debug_address();
    DEBUG_KPROBE_LIST.lock().insert(debug_address, kprobe);
    detect_func(1, 2);

    BREAK_KPROBE_LIST.lock().remove(&(detect_func as usize));
    DEBUG_KPROBE_LIST.lock().remove(&debug_address);
    detect_func(1, 2);
}
