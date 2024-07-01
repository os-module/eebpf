
use core::sync::atomic::AtomicBool;


#[repr(C)]
pub struct StaticKey{
    enabled: AtomicBool,
    entries: *const JumpEntry,
}

impl StaticKey{
}

const STATIC_KEY_INIT_TRUE: StaticKey = StaticKey{
    enabled: AtomicBool::new(true),
    entries: 0 as *const JumpEntry,
};


const STATIC_KEY_INIT_FALSE: StaticKey = StaticKey{
    enabled: AtomicBool::new(false),
    entries: 0 as *const JumpEntry,
};

struct StaticKeyTrue(StaticKey);

struct StaticKeyFalse(StaticKey);


#[macro_export]
macro_rules! static_key_true_init {
    () => {
        StaticKeyTrue(STATIC_KEY_INIT_TRUE)
    };
}

#[macro_export]
macro_rules! static_key_false_init {
    () => {
        StaticKeyFalse(STATIC_KEY_INIT_FALSE)
    };
}


#[macro_export]
macro_rules! define_static_key_true {
    ($name:ident) => {
        static $name: StaticKeyTrue = static_key_true_init!();
    };
}

#[macro_export]
macro_rules! define_static_key_false {
    ($name:ident) => {
        static $name: StaticKeyFalse = static_key_false_init!();
    };
}

macro_rules!  static_branch_likely {
    ($key:expr) => {
        let branch:bool;
        if core::intrinsics::type_id
    };
}


#[repr(C)]
pub struct JumpEntry{
    code: u32,
    addr: u32,
    key_addr: u64,
}



