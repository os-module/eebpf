#![no_std]
#![feature(specialization)]
#![feature(riscv_ext_intrinsics)]
#![allow(incomplete_features)]

mod arch;
use core::{fmt::Debug, sync::atomic::AtomicBool};

pub use arch::*;

pub const BRANCH_TRUE: usize = 1;
pub const BRANCH_FALSE: usize = 0;

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

    pub unsafe fn from_raw_addr(addr: usize) -> &'static StaticKey {
        &*(addr as *const Self)
    }

    pub unsafe fn from_raw_addr_mut(addr: usize) -> &'static mut StaticKey {
        &mut *(addr as *mut Self)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled
            .store(enabled, core::sync::atomic::Ordering::Relaxed)
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
    #[inline]
    default fn static_key_type(&self) -> StaticKeyType {
        StaticKeyType::Other
    }
}

impl StaticKeyTypeTrait for StaticKeyTrue {
    #[inline]
    fn static_key_type(&self) -> StaticKeyType {
        StaticKeyType::StaticKeyTrue
    }
}

impl StaticKeyTypeTrait for StaticKeyFalse {
    #[inline]
    fn static_key_type(&self) -> StaticKeyType {
        StaticKeyType::StaticKeyFalse
    }
}

#[macro_export]
macro_rules! static_branch_likely {
    ($key:ident) => {{
        if $key.static_key_type() == StaticKeyType::StaticKeyTrue {
            // !arch_static_branch!($key, BRANCH_TRUE)
            unsafe {
                paste! {
                    [<$key _is_false>]()
                }
            }
        } else if $key.static_key_type() == StaticKeyType::StaticKeyFalse {
            // !arch_static_branch_jump!($key, BRANCH_FALSE)
            unsafe {
                paste! {
                    [<$key _is_false>]()
                }
            }
        } else {
            panic!("static key is not true or false")
        }
    }};
}

#[macro_export]
macro_rules! static_branch_unlikely {
    ($key:ident) => {{
        if $key.static_key_type() == StaticKeyType::StaticKeyTrue {
            // arch_static_branch_jump!($key, BRANCH_FALSE)
            unsafe {
                paste! {
                    ![<$key _is_false>]()
                }
            }
        } else if $key.static_key_type() == StaticKeyType::StaticKeyFalse {
            // arch_static_branch!($key, BRANCH_FALSE)
            unsafe {
                paste! {
                    ![<$key _is_false>]()
                }
            }
        } else {
            panic!("static key is not true or false")
        }
    }};
}

#[macro_export]
macro_rules! static_branch_enable {
    ($key:ident) => {{
        static_key_enable(
            &$key.0,
            paste! {
                    [<$key _is_false>]
            } as usize,
        )
    }};
}

#[macro_export]
macro_rules! static_branch_disable {
    ($key:ident) => {{
        static_key_disable(
            &$key.0,
            paste! {
                    [<$key _is_false>]
            } as usize,
        )
    }};
}

#[repr(C)]
#[derive(Debug)]
pub struct JumpEntry {
    code: usize,
    addr: usize,
    key_addr: usize,
}

impl JumpEntry {
    pub fn new(code: usize, addr: usize, key_addr: usize) -> Self {
        JumpEntry {
            code,
            addr,
            key_addr,
        }
    }
    /// The address of the `nop` instruction
    pub fn code_addr(&self) -> usize {
        self.code
    }

    /// The address of the target function which will return `true`
    pub fn target_addr(&self) -> usize {
        self.addr
    }

    /// The address of the static key
    pub fn static_key_addr(&self) -> usize {
        self.key_addr & (!0x1)
    }

    pub fn is_branch(&self) -> bool {
        self.key_addr & 0x1 == 1
    }
}

pub enum JumpLabelType {
    Nop,
    Jmp,
}
