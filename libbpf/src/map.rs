use alloc::{collections::BTreeMap, vec::Vec};

#[derive(Debug, Clone)]
pub struct MapEntry {
    data: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct MapKey {
    data: Vec<u8>,
}

impl MapEntry {
    pub fn new(data: Vec<u8>) -> Self {
        MapEntry { data }
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl MapKey {
    pub fn new(data: Vec<u8>) -> Self {
        MapKey { data }
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[derive(Debug)]
pub struct BpfMap {
    map: BTreeMap<MapKey, MapEntry>,
    max_entries: u32,
    key_size: u32,
    value_size: u32,
}

impl BpfMap {
    pub const fn new(key_size: u32, value_size: u32, max_entries: u32) -> Self {
        let map = BpfMap {
            map: BTreeMap::new(),
            max_entries,
            key_size,
            value_size,
        };
        map
    }
    pub fn insert(&mut self, key: MapKey, value: MapEntry) {
        assert_eq!(key.data().len() as u32, self.key_size);
        assert_eq!(value.data().len() as u32, self.value_size);
        self.map.insert(key, value);
    }
    pub fn get(&self, key: &MapKey) -> Option<&MapEntry> {
        self.map.get(key)
    }

    pub fn get_mut(&mut self, key: &MapKey) -> Option<&mut MapEntry> {
        self.map.get_mut(key)
    }

    pub fn remove(&mut self, key: &MapKey) -> Option<MapEntry> {
        self.map.remove(key)
    }

    pub fn len(&self) -> usize {
        self.max_entries as usize * self.value_size as usize
    }

    pub fn key_size(&self) -> u32 {
        self.key_size
    }

    pub fn value_size(&self) -> u32 {
        self.value_size
    }

    pub fn update(&mut self, key: &MapKey, value: &MapEntry) {
        assert_eq!(key.data().len() as u32, self.key_size);
        assert_eq!(value.data().len() as u32, self.value_size);
        let entry = self.map.get_mut(key).unwrap();
        entry.data_mut().copy_from_slice(value.data());
    }
}
