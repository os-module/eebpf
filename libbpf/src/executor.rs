use alloc::vec::Vec;

use crate::loader::Relocation;
use crate::INS_SIZE;
use anyhow::Result;
use rbpf::{ebpf, ebpf::to_insn_vec};

pub struct BpfExecutor<F> {
    _f: core::marker::PhantomData<F>,
}

pub trait FindMapOps {
    fn map_data_len(map_fd: usize) -> usize;
    fn map_data_ptr(map_fd: usize) -> *mut u8;
}

impl<F: FindMapOps> BpfExecutor<F> {
    pub fn process(prog: &[u8], relocations: &[Relocation]) -> Result<Vec<u8>> {
        let mut instructions = to_insn_vec(prog);
        // we need update the LD_IMM64 instruction
        for relocation in relocations {
            let index = relocation.offset / INS_SIZE;
            let mut insn = instructions[index].clone();
            if insn.opc == ebpf::LD_DW_IMM {
                let mut next_insn = instructions[index + 1].clone();
                // Now the imm is the map_fd
                let imm = insn.imm as usize;
                let map_data_ptr = F::map_data_ptr(imm) as usize;
                let map_data_offset = next_insn.imm as usize;
                // The current ins store the map_data_ptr low 32 bits,
                // the next ins store the map_data_ptr high 32 bits
                insn.imm = (map_data_ptr + map_data_offset) as i32;
                next_insn.imm = ((map_data_ptr + map_data_offset) >> 32) as i32;
                instructions[index] = insn;
                instructions[index + 1] = next_insn;
            }
        }
        let prog = instructions
            .iter()
            .map(|ins| ins.to_vec())
            .flatten()
            .collect();
        Ok(prog)
    }
}
