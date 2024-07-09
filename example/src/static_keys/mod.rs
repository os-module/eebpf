// extern "C"{
//     fn __start_jump_table();
//     fn __end_jump_table();
// }

// #[no_mangle]
// #[inline(never)]
// pub fn test_static_keys(_panic:bool){
//     println!("test_static_keys");
//     let jump_table_size = unsafe { __end_jump_table as usize - __start_jump_table as usize };
//     println!("jump_table_size: {}", jump_table_size);
//     let entry_size = core::mem::size_of::<JumpEntry>();
//     let entry = unsafe { core::slice::from_raw_parts(__start_jump_table as *mut JumpEntry, jump_table_size / entry_size) };
//     println!("entry: {:#x?}", entry);
//     // println!("label: {:#x?}", label as usize);
// }
