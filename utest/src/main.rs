use std::{collections::BTreeMap, fmt::Write, path::PathBuf};

use anyhow::Result;
use libbpf::{
    executor::{BpfExecutor, FindMapOps},
    loader::{BpfLoader, BpfMapAttr, CreateMapOps, MapFd},
    map::{BpfMap, MapEntry, MapKey},
    print::printf_with,
};
use rbpf::{disassembler, helpers};
use spin::Mutex;

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("trace"));
    let filename = PathBuf::from("./libbpf/bpf/hello.bpf.o");
    let file_data = std::fs::read(filename).expect("Could not read file.");
    let slice = file_data.as_slice();

    let loader = BpfLoader::<CreateMapFdImpl>::new()
        .elf(slice)
        .text_section_name("xdp")
        .load()
        .unwrap();

    let prog = loader.text();
    log::info!("After the pre-processing, the program is:");

    disassembler::disassemble(&prog);

    let new_prog = BpfExecutor::<FindMapImpl>::process(prog).unwrap();

    log::info!("After the post-processing, the program is:");
    disassembler::disassemble(&new_prog);

    let hkey = helpers::BPF_TRACE_PRINTK_IDX as u8;
    let mut vm = rbpf::EbpfVmRaw::new(Some(&new_prog)).unwrap();
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
    unsafe { printf_with(&mut FakeOut, fmt_ptr as _, arg3, arg4, arg5) as u64 }
}

static BPF_MAP: Mutex<BTreeMap<MapFd, BpfMap>> = Mutex::new(BTreeMap::new());
static BPF_MAP_ID: Mutex<MapFd> = Mutex::new(1);
pub struct CreateMapFdImpl;
pub struct FindMapImpl;

impl CreateMapOps for CreateMapFdImpl {
    fn create_map(attr: BpfMapAttr) -> Result<MapFd> {
        let key_size = attr.key_size;
        let value_size = attr.value_size;
        let max_entries = attr.max_entries;
        let _name = attr.name;
        let mut map = BpfMap::new(key_size, value_size, max_entries);
        if max_entries == 1 {
            map.insert(
                MapKey::new(vec![0; key_size as usize]),
                MapEntry::new(vec![0; value_size as usize]),
            );
        }
        let mut map_id = BPF_MAP_ID.lock();
        let map_fd = *map_id;
        *map_id += 1;
        BPF_MAP.lock().insert(map_fd, map);
        Ok(map_fd)
    }
    fn map_data_len(map_fd: MapFd) -> usize {
        BPF_MAP.lock().get(&map_fd).unwrap().len()
    }
    fn update_map_element(map_fd: MapFd, key: &[u8], value: &[u8]) -> Result<()> {
        log::info!("update map element: {:?} with value: {:?}", key, value);
        let mut map = BPF_MAP.lock();
        let map = map.get_mut(&map_fd).unwrap();
        map.update(&MapKey::new(key.to_vec()), &MapEntry::new(value.to_vec()));
        Ok(())
    }
}

impl FindMapOps for FindMapImpl {
    fn map_data_len(map_fd: usize) -> usize {
        BPF_MAP.lock().get(&map_fd).unwrap().len()
    }
    fn map_data_ptr(map_fd: usize) -> *mut u8 {
        let mut map = BPF_MAP.lock();
        let map = map
            .get_mut(&map_fd)
            .expect(format!("map with fd {} not found", map_fd).as_str());
        let value = map
            .get_mut(&MapKey::new(vec![0; map.key_size() as usize]))
            .unwrap();
        let value_ptr = value.data_mut().as_mut_ptr();
        log::info!("get map data ptr: {:p}", value_ptr);
        value_ptr
    }
}
