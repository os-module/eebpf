fn main() {
    // This is the eBPF program, in the form of bytecode instructions.
    let prog = &[
        0xb4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov32 r0, 0
        0xb4, 0x01, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, // mov32 r1, 2
        0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, // add32 r0, 1
        0x0c, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // add32 r0, r1
        0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
    ];

    // Instantiate a struct EbpfVmNoData. This is an eBPF VM for programs that
    // takes no packet data in argument.
    // The eBPF program is passed to the constructor.
    let vm = rbpf::EbpfVmNoData::new(Some(prog)).unwrap();

    // Execute (interpret) the program. No argument required for this VM.
    assert_eq!(vm.execute_program().unwrap(), 0x3);

    let prog = &[
        0x71, 0x10, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, // ldxh r0, [r1+2]
        0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
    ];

    // Let's use some data.
    let mem = &mut [0xaa, 0xbb, 0x11, 0xcc, 0xdd];

    // This is an eBPF VM for programs reading from a given memory area (it
    // directly reads from packet data)
    let mut vm = rbpf::EbpfVmRaw::new(Some(prog)).unwrap();

    #[cfg(windows)]
    {
        assert_eq!(vm.execute_program(mem).unwrap(), 0x11);
    }
    #[cfg(not(windows))]
    {
        // This time we JIT-compile the program.
        vm.jit_compile().unwrap();

        // Then we execute it. For this kind of VM, a reference to the packet
        // data must be passed to the function that executes the program.
        unsafe {
            assert_eq!(vm.execute_program_jit(mem).unwrap(), 0x11);
        }
    }

    let decoder = yaxpeax_x86::amd64::InstDecoder::default();

    let inst = decoder.decode_slice(&[0x48, 0x83, 0xec, 0x28]).unwrap();
}
