use core::fmt::{Debug, Formatter, Write};
use riscv::register::sstatus::Sstatus;

#[repr(C)]
pub struct TrapFrame {
    pub pt_regs: PtRegs,
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl Debug for TrapFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("sepc: {:#x}\n", self.sepc))?;
        f.write_fmt(format_args!("{:?}", self.pt_regs))?;
        Ok(())
    }
}
#[repr(C)]
pub struct PtRegs {
    pub x: [usize; 32],
}

impl Debug for PtRegs {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "x0: {:#x}, ra: {:#x}, sp: {:#x}, gp: {:#x}\n",
            self.x[0], self.x[1], self.x[2], self.x[3]
        ))?;
        f.write_fmt(format_args!(
            "tp: {:#x}, t0: {:#x}, t1: {:#x}, t2: {:#x}\n",
            self.x[4], self.x[5], self.x[6], self.x[7]
        ))?;
        f.write_fmt(format_args!(
            "fp: {:#x}, s1: {:#x}, a0: {:#x}, a1: {:#x}\n",
            self.x[8], self.x[9], self.x[10], self.x[11]
        ))?;
        f.write_fmt(format_args!(
            "a2: {:#x}, a3: {:#x}, a4: {:#x}, a5: {:#x}\n",
            self.x[12], self.x[13], self.x[14], self.x[15]
        ))?;
        f.write_fmt(format_args!(
            "a6: {:#x}, a7: {:#x}, s2: {:#x}, s3: {:#x}\n",
            self.x[16], self.x[17], self.x[18], self.x[19]
        ))?;
        f.write_fmt(format_args!(
            "s4: {:#x}, s5: {:#x}, s6: {:#x}, s7: {:#x}\n",
            self.x[20], self.x[21], self.x[22], self.x[23]
        ))?;
        f.write_fmt(format_args!(
            "s8: {:#x}, s9: {:#x}, s10: {:#x}, s11: {:#x}\n",
            self.x[24], self.x[25], self.x[26], self.x[27]
        ))?;
        f.write_fmt(format_args!(
            "t3: {:#x}, t4: {:#x}, t5: {:#x}, t6: {:#x}",
            self.x[28], self.x[29], self.x[30], self.x[31]
        ))?;
        Ok(())
    }
}
