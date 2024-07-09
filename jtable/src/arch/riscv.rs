const RISCV_INSN_NOP: u32 = 0x00000013;
const RISCV_INSN_JAL: u32 = 0x0000006F;

const JUMP_LABEL_NOP_SIZE: usize = 4;

pub fn jal_to_offset(off: i32) -> u32 {
    let mut insn = RISCV_INSN_JAL;
    if off < -524288 || off > 524288 {
        panic!("jal offset out of range: {}", off);
    };
    let off = off as u32;
    insn |= (off & 0x80000000) >> 11;
    insn |= (off & 0x000007FE) << 1;
    insn |= (off & 0x00000800) << 20;
    insn |= (off & 0x7FE00000) >> 9;
    insn |= (off & 0x80000000) >> 20;
    insn
}

#[macro_export]
macro_rules! arch_static_branch {
    ($name:ident, $branch:ident) => {
        unsafe{
            #[inline(always)]
            fn label()->bool{
                return true;
            }
            core::arch::asm!(
                "
                .option push
                .option norelax
                .option norvc
                1: nop
                .option pop
                .pushsection __jump_table, \"aw\"
                .align 3
                .quad 1b, {target}, {name} + {branch}
                .popsection
                ",
                target = sym label,
                name = sym $name,
                branch = const $branch,
            );
            false
        }
    };
}

#[macro_export]
macro_rules! arch_static_branch_jump{
    ($name:ident, $branch:ident) => {
        unsafe{
            #[inline(always)]
            fn label()->bool{
                return true;
            }
            core::arch::asm!(
                "
                .option push
                .option norelax
                .option norvc
                1:
                call {target}
                .option pop
                .pushsection __jump_table, \"aw\"
                .align 3
                .quad 1b, {target}, {name} + {branch}
                .popsection
                ",
                target = sym label,
                name = sym $name,
                branch = const $branch,
            );
            false
        }
    };
}
