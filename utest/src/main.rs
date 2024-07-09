use std::{fmt::Write, path::PathBuf};

use elf::{endian::AnyEndian, section::SectionHeader, ElfBytes};
use libbpf::printf_with;
use rbpf::{disassembler, helpers};

fn main() {
    let hkey = helpers::BPF_TRACE_PRINTK_IDX as u8;
    let prog = &[
        0x85, 0x00, 0x00, 0x00, hkey, 0x00, 0x00, 0x00, // call helper <hkey>
        0x71, 0x10, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, // ldxh r0, [r1+2]
        0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
    ];
    // Let's use some data.
    let mem = &mut [0xaa, 0xbb, 0x11, 0xcc, 0xdd];
    // This is an eBPF VM for programs reading from a given memory area (it
    // directly reads from packet data)
    let mut vm = rbpf::EbpfVmRaw::new(Some(prog)).unwrap();
    vm.register_helper(hkey as u32, helpers::bpf_trace_printf)
        .unwrap();
    assert_eq!(vm.execute_program(mem).unwrap(), 0x11);

    let filename = PathBuf::from("./libbpf/bpf/hello.bpf.o");
    let file_data = std::fs::read(filename).expect("Could not read file.");
    let slice = file_data.as_slice();
    let file = ElfBytes::<AnyEndian>::minimal_parse(slice).expect("Open test1");
    // Get the ELF file's build-id
    let xdp: SectionHeader = file
        .section_header_by_name("xdp")
        .expect("section table should be parseable")
        .expect("file should have a xdp section");
    let (data, _) = file.section_data(&xdp).unwrap();
    let prog = data.to_vec();

    disassembler::disassemble(&prog);
    println!("------------------------");
    let mut vm = rbpf::EbpfVmRaw::new(Some(&prog)).unwrap();
    vm.register_helper(hkey as u32, trace_printf).unwrap();
    let res = vm.execute_program(&mut []).unwrap();
    println!("Program returned: {res:?} ({res:#x})");
    println!("Test passed!");
}

struct FakeOut;
impl Write for FakeOut {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}

pub fn trace_printf(fmt_ptr: u64, _fmt_len: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    // println!("bpf_trace_printf: {fmt_ptr:#x} {fmt_len:#x} {arg3:#x}, {arg4:#x}, {arg5:#x}");
    unsafe { printf_with(&mut FakeOut, fmt_ptr as _, arg3, arg4, arg5) as u64 }
}
