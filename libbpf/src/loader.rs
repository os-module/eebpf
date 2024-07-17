use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::marker::PhantomData;

use anyhow::Result;
use elf::{endian::AnyEndian, ElfBytes};
use rbpf::ebpf::{to_insn_vec, Insn};

#[derive(Debug)]
pub struct BpfLoader<'data, C> {
    elf_data: Option<ElfBytes<'data, AnyEndian>>,
    text_section_name: Option<String>,
    _c: PhantomData<C>,
}

pub struct BpfMapAttr {
    pub map_type: u32,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub map_flags: u32,
    pub name: String,
}

pub trait CreateMapOps {
    fn create_map(attr: BpfMapAttr) -> Result<MapFd>;
    fn map_data_len(map_fd: MapFd) -> usize;
    fn update_map_element(map_fd: MapFd, key: &[u8], value: &[u8]) -> Result<()>;
}

const INS_SIZE: usize = 8;
#[derive(Debug)]
pub struct Relocation {
    pub offset: usize,
    pub symbol: String,
    pub section_index: usize,
    pub section_name: String,
    pub section_addr: u64,
    pub ty: u32,
}

type SecIndex = usize;
pub type MapFd = usize;

impl<'data, C: CreateMapOps> BpfLoader<'data, C> {
    pub fn new() -> Self {
        BpfLoader {
            elf_data: None,
            text_section_name: None,
            _c: PhantomData,
        }
    }

    pub fn text_section_name(mut self, name: &str) -> Self {
        self.text_section_name = Some(name.to_string());
        self
    }

    pub fn elf(mut self, elf: &'data [u8]) -> Self {
        self.elf_data =
            Some(ElfBytes::<AnyEndian>::minimal_parse(elf).expect("The elf file is not valid"));
        self
    }

    fn create_map(&mut self) -> Result<BTreeMap<SecIndex, MapFd>> {
        let elf = self.elf_data.as_ref().unwrap();
        let (section_headers, section_headers_name_table) =
            elf.section_headers_with_strtab().unwrap();
        let (section_headers, section_headers_name_table) = (
            section_headers.unwrap(),
            section_headers_name_table.unwrap(),
        );
        let mut map_fds = BTreeMap::new();
        for (idx, section) in section_headers.iter().enumerate() {
            let name_index = section.sh_name;
            let name = section_headers_name_table.get(name_index as usize).unwrap();
            let section_size = section.sh_size;
            if name.starts_with(".bss") {
                let map_attr = BpfMapAttr {
                    map_type: 1,
                    key_size: 4,
                    value_size: section_size as u32,
                    max_entries: 1,
                    map_flags: 0,
                    name: name.to_string(),
                };
                let map_fd = C::create_map(map_attr)?;
                map_fds.insert(idx, map_fd);
                log::info!(
                    "create map for section: {} with size: {}, map_fd: {}",
                    name,
                    section_size,
                    map_fd
                );
            } else if name.starts_with(".data") {
                let section_size = section.sh_size;
                let map_attr = BpfMapAttr {
                    map_type: 2,
                    key_size: 4,
                    value_size: section_size as u32,
                    max_entries: 1,
                    map_flags: 0,
                    name: name.to_string(),
                };
                let map_fd = C::create_map(map_attr)?;
                map_fds.insert(idx, map_fd);
                log::info!(
                    "create map for section: {} with size: {}, map_fd: {}",
                    name,
                    section_size,
                    map_fd
                );
                log::info!("The section is .Data, we need update the map data");
                let (data, _) = elf.section_data(&section).unwrap();
                assert_eq!(data.len(), section_size as usize);
                let key = [0; 4];
                C::update_map_element(map_fd, &key, data)?;
            }
        }
        Ok(map_fds)
    }

    fn find_prog(&self) -> Result<Vec<Insn>> {
        let elf = self.elf_data.as_ref().unwrap();
        let text_section_name = self.text_section_name.as_ref().unwrap();
        let text = elf
            .section_header_by_name(text_section_name)
            .expect("section table should be parseable")
            .expect("file should have a xdp section");
        let (prog_data, _) = elf.section_data(&text).unwrap();
        let insn = to_insn_vec(prog_data);
        Ok(insn)
    }

    fn relocations(&self, section_name: &str) -> Result<Vec<Relocation>> {
        let elf = self.elf_data.as_ref().unwrap();
        let section = elf
            .section_header_by_name(&section_name)
            .expect("section table should be parseable")
            .expect(&format!("file should have a {} section", section_name));

        let relocation_section = elf
            .section_data_as_rels(&section)
            .expect("Failed to parse relocations");

        let (section_headers, section_headers_name_table) =
            elf.section_headers_with_strtab().unwrap();
        let (section_headers, section_headers_name_table) = (
            section_headers.unwrap(),
            section_headers_name_table.unwrap(),
        );

        let (symbol_table, string_table) = elf.symbol_table().unwrap().unwrap();

        let mut relocations = Vec::new();
        relocation_section.for_each(|item| {
            log::info!("{:?}", item);
            let symbol = symbol_table.get(item.r_sym as usize).unwrap();
            let name = string_table.get(symbol.st_name as usize).unwrap();
            let section_index = symbol.st_shndx;
            let section_header = section_headers.get(section_index as usize).unwrap();
            let section_header_name_index = section_header.sh_name;
            let section_header_name = section_headers_name_table
                .get(section_header_name_index as usize)
                .expect(
                    format!(
                        "section header name index {} is invalid",
                        section_header_name_index
                    )
                    .as_str(),
                );
            log::info!(
                "name: {} -> [{}] {:?} ",
                name,
                section_index,
                section_header_name
            );
            let relocation = Relocation {
                offset: item.r_offset as usize,
                symbol: name.to_string(),
                section_index: section_index as usize,
                section_name: section_header_name.to_string(),
                section_addr: section_header.sh_addr,
                ty: item.r_type,
            };
            relocations.push(relocation);
        });
        Ok(relocations)
    }

    pub fn load(mut self) -> Result<Bpf> {
        let bpf_map = self.create_map()?;
        let mut prog = self.find_prog()?;

        let text_section_name = self.text_section_name.as_ref().unwrap();
        let text_relocation_name = format!(".rel{}", text_section_name);

        let relocations = self.relocations(&text_relocation_name)?;

        for relocation in relocations {
            let ins_index = relocation.offset / INS_SIZE;

            let map_fd = bpf_map.get(&relocation.section_index).expect(
                format!(
                    "map not found for section index: {}",
                    relocation.section_index
                )
                .as_str(),
            );
            let map_data_len = C::map_data_len(*map_fd);

            let instructions = prog.as_mut_slice();
            if map_data_len != 0 {
                log::error!(
                    "relocate_maps: map data is not empty, set src_reg to BPF_PSEUDO_MAP_VALUE: {}",
                    BPF_PSEUDO_MAP_VALUE
                );
                log::error!(
                    "relocate_maps: set next imm to sym.address:{} + ins.imm:{} = {}",
                    relocation.section_addr,
                    instructions[ins_index].imm,
                    relocation.section_addr + instructions[ins_index].imm as u64
                );
                instructions[ins_index].set_src_reg(BPF_PSEUDO_MAP_VALUE as u8);
                instructions[ins_index + 1].imm =
                    instructions[ins_index].imm + relocation.section_addr as i32;
            } else {
                log::error!(
                    "relocate_maps: map data is empty, set src_reg to BPF_PSEUDO_MAP_FD: {}",
                    BPF_PSEUDO_MAP_FD
                );
                instructions[ins_index].set_src_reg(BPF_PSEUDO_MAP_FD as u8);
            }
            log::error!("relocate_maps: set imm to fd: {}", map_fd);
            instructions[ins_index].imm = *map_fd as i32;
        }

        let prog = prog.iter().map(|ins| ins.to_vec()).flatten().collect();

        Ok(Bpf {
            text: prog,
            bpf_map,
        })
    }
}

pub const BPF_PSEUDO_MAP_FD: u32 = 1;
pub const BPF_PSEUDO_MAP_IDX: u32 = 5;
pub const BPF_PSEUDO_MAP_VALUE: u32 = 2;

pub trait InsExt {
    fn set_src_reg(&mut self, src: u8);
}

impl InsExt for Insn {
    fn set_src_reg(&mut self, src: u8) {
        self.src = src;
    }
}

#[derive(Debug)]
pub struct Bpf {
    text: Vec<u8>,
    bpf_map: BTreeMap<SecIndex, MapFd>,
}

impl Bpf {
    pub fn text(&self) -> &[u8] {
        &self.text
    }
}
