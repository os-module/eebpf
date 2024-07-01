#![feature(specialization)]
#![allow(incomplete_features)]
use core::fmt::Debug;
use core::sync::atomic::AtomicBool;
#[repr(C)]
#[derive(Debug)]
pub struct StaticKey {
    enabled: AtomicBool,
    entries: *const JumpEntry,
}

impl StaticKey {
    pub const fn default_true() -> Self {
        StaticKey {
            enabled: AtomicBool::new(true),
            entries: 0 as *const JumpEntry,
        }
    }
    pub const fn default_false() -> Self {
        StaticKey {
            enabled: AtomicBool::new(false),
            entries: 0 as *const JumpEntry,
        }
    }
}

unsafe impl Send for StaticKey {}
unsafe impl Sync for StaticKey {}

#[derive(Debug)]
pub struct StaticKeyTrue(pub StaticKey);

impl StaticKeyTrue {
    pub const fn new() -> Self {
        StaticKeyTrue(StaticKey::default_true())
    }
}

#[derive(Debug)]
pub struct StaticKeyFalse(pub StaticKey);

impl StaticKeyFalse {
    pub const fn new() -> Self {
        StaticKeyFalse(StaticKey::default_false())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum StaticKeyType {
    StaticKeyTrue,
    StaticKeyFalse,
    Other,
}

pub trait StaticKeyTypeTrait {
    fn static_key_type(&self) -> StaticKeyType;
}

impl<T> StaticKeyTypeTrait for T {
    default fn static_key_type(&self) -> StaticKeyType {
        StaticKeyType::Other
    }
}

impl StaticKeyTypeTrait for StaticKeyTrue {
    fn static_key_type(&self) -> StaticKeyType {
        StaticKeyType::StaticKeyTrue
    }
}

impl StaticKeyTypeTrait for StaticKeyFalse {
    fn static_key_type(&self) -> StaticKeyType {
        StaticKeyType::StaticKeyFalse
    }
}

#[macro_export]
macro_rules! define_static_key_true {
    ($name:ident) => {
        static $name: StaticKeyTrue = StaticKeyTrue::new();
    };
}

#[macro_export]
macro_rules! define_static_key_false {
    ($name:ident) => {
        static $name: StaticKeyFalse = StaticKeyFalse::new();
    };
}
#[macro_export]
macro_rules! static_branch_likely {
    ($key:expr) => {{
        // let bool
        if $key.static_key_type() == StaticKeyType::StaticKeyTrue {
            test_static_key_true();
        } else if $key.static_key_type() == StaticKeyType::StaticKeyFalse {
            test_static_key_false();
        } else {
            test_static_other();
        }
    }};
}

#[inline(never)]
pub fn test_static_key_true() {
    println!("test_static_key_true");
}

#[inline(never)]
pub fn test_static_key_false() {
    println!("test_static_key_false");
}

#[inline(never)]
pub fn test_static_other() {
    println!("test_static_other");
}

#[repr(C)]
#[derive(Debug)]
pub struct JumpEntry {
    code: u32,
    addr: u32,
    key_addr: u64,
}
