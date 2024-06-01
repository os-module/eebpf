use crate::arch::hart_id;
use crate::trap::context::PtRegs;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::fmt::Debug;
use spin::Mutex;

pub static KPROBE_LIST: Mutex<BTreeMap<usize, Kprobe>> = Mutex::new(BTreeMap::new());

pub struct Kprobe {
    symbol: String,
    symbol_addr: usize,
    offset: usize,
    old_instruction: u32,
    pre_handler: Box<dyn Fn(&PtRegs)>,
    post_handler: Box<dyn Fn(&PtRegs)>,
    fault_handler: Box<dyn Fn(&PtRegs)>,
    inst_tmp: [u8; 8],
}

impl Debug for Kprobe {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Kprobe")
            .field("symbol", &self.symbol)
            .field("offset", &self.offset)
            .finish()
    }
}

unsafe impl Send for Kprobe {}

impl Kprobe {
    pub fn new(
        symbol: String,
        symbol_addr: usize,
        offset: usize,
        pre_handler: Box<dyn Fn(&PtRegs)>,
        post_handler: Box<dyn Fn(&PtRegs)>,
        fault_handler: Box<dyn Fn(&PtRegs)>,
    ) -> Self {
        Kprobe {
            symbol,
            symbol_addr,
            offset,
            old_instruction: 0,
            pre_handler,
            post_handler,
            fault_handler,
            inst_tmp: [0; 8],
        }
    }

    pub fn install(&mut self) {
        let address = self.symbol_addr + self.offset;
        self.old_instruction = unsafe { core::ptr::read(address as *const u32) };
        let ebreak_inst = 0b00000000000100000_000_00000_1110011u32;
        unsafe {
            core::ptr::write(address as *mut u32, ebreak_inst);
        }
        println!(
            "Kprobe::install: address: {:#x}, func_name: {}",
            address, self.symbol
        );
    }

    pub fn uninstall(&mut self) -> usize {
        let address = self.symbol_addr + self.offset;
        unsafe {
            core::ptr::write(address as *mut u32, self.old_instruction);
        }
        let old_instruction = self.old_instruction;
        let is_inst_16 = match old_instruction & 0x3 {
            0 | 1 | 2 => true,
            _ => false,
        };
        println!(
            "Kprobe::uninstall: address: {:#x}, old_instruction: {:#x}",
            address, self.old_instruction
        );
        if is_inst_16 {
            address + 2
        } else {
            address + 4
        }
    }

    pub fn pre_handler(&self, regs: &PtRegs) {
        (self.pre_handler)(regs);
    }

    pub fn post_handler(&self, regs: &PtRegs) {
        (self.post_handler)(regs);
    }

    pub fn fault_handler(&self, regs: &PtRegs) {
        (self.fault_handler)(regs);
    }

    pub fn old_inst(&self) -> u32 {
        self.old_instruction
    }

    pub fn simulate_single_step(&mut self) -> (usize, usize) {
        let old_instruction = self.old_instruction;
        let is_inst_16 = match old_instruction & 0x3 {
            0 | 1 | 2 => true,
            _ => false,
        };
        let ebreak_inst = 0b00000000000100000_000_00000_1110011u32;
        let inst_tmp_ptr = self.inst_tmp.as_ptr() as usize;
        if is_inst_16 {
            let inst_16 = old_instruction as u16;
            unsafe {
                // inst_16 :0-16
                // ebreak  :16-32
                core::ptr::write(inst_tmp_ptr as *mut u16, inst_16);
                core::ptr::write((inst_tmp_ptr + 2) as *mut u32, ebreak_inst);
            }
            (inst_tmp_ptr, inst_tmp_ptr + 2)
        } else {
            unsafe {
                // inst_32 :0-32
                // ebreak  :32-64
                core::ptr::write(inst_tmp_ptr as *mut u32, old_instruction);
                core::ptr::write((inst_tmp_ptr + 4) as *mut u32, ebreak_inst);
            }
            (inst_tmp_ptr, inst_tmp_ptr + 4)
        }
    }
}

#[no_mangle]
pub fn detect_func(x: usize, y: usize) -> usize {
    let hart = hart_id();
    println!("detect_func: hart_id: {}, x: {}", hart, x);
    hart
}

pub fn test_kprobe() {
    let pre_handler = Box::new(|regs: &PtRegs| {
        println!("call pre_handler");
    });
    let post_handler = Box::new(|regs: &PtRegs| {
        println!("call post_handler");
    });
    let fault_handler = Box::new(|regs: &PtRegs| {
        println!("call fault_handler");
    });
    let mut kprobe = Kprobe::new(
        "detect_func".to_string(),
        detect_func as usize,
        0,
        pre_handler,
        post_handler,
        fault_handler,
    );

    kprobe.install();

    KPROBE_LIST.lock().insert(detect_func as usize, kprobe);

    detect_func(1, 2);
}
