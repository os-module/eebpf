use crate::sbi::system_shutdown;
use crate::{main, println};
use core::arch::asm;
use spin::Mutex;
use talc::{ClaimOnOom, Span, Talc, Talck};

/// 内核启动栈大小
pub const STACK_SIZE: usize = 1024 * 64;
/// 内核启动栈大小的位数
pub const STACK_SIZE_BITS: usize = 16;
const KERNEL_HEAP_SIZE: usize = 0x26_00000;
/// 可配置的启动cpu数量
pub const CPU_NUM: usize = 1;
#[link_section = ".bss.stack"]
static mut STACK: [u8; STACK_SIZE * CPU_NUM] = [0; STACK_SIZE * CPU_NUM];

#[global_allocator]
static HEAP_ALLOCATOR: Talck<Mutex<()>, ClaimOnOom> =
    Talc::new(unsafe { ClaimOnOom::new(Span::from_const_array(core::ptr::addr_of!(KERNEL_HEAP))) })
        .lock();
static mut KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// 内核入口
///
/// 用于初始化内核的栈空间，并关闭中断
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
extern "C" fn _start() {
    unsafe {
        asm!("\
        mv tp, a0
        mv gp, a1
        add t0, a0, 1
        slli t0, t0, {stack_size_bits}
        la sp, {boot_stack}
        add sp, sp, t0
        mv a0, tp
        mv a1, gp
        call {platform_init}
        ",
        stack_size_bits = const STACK_SIZE_BITS,
        boot_stack = sym STACK,
        platform_init = sym platform_init,
        options(noreturn)
        );
    }
}

extern "C" {
    fn sbss();
    fn ebss();
}

/// 清空.bss段
fn clear_bss() {
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

pub const ALIEN_FLAG: &str = r"
     _      _   _
    / \    | | (_)   ___   _ __
   / _ \   | | | |  / _ \ | '_ \
  / ___ \  | | | | |  __/ | | | |
 /_/   \_\ |_| |_|  \___| |_| |_|
";
pub fn platform_init(hart_id: usize, dtb: usize) {
    clear_bss();
    println!("{}", ALIEN_FLAG);
    main();
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    system_shutdown();
}
