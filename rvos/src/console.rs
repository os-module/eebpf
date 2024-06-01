use core::fmt::{Arguments, Result, Write};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let hard_id = crate::arch::hart_id();
        $crate::console::__print(format_args!("[{}] {}", hard_id, format_args!($($arg)*)))
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! iprint {
    ($($arg:tt)*) => {
        $crate::console::__print(format_args!("{}", format_args!($($arg)*)))
    };
}

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> Result {
        s.as_bytes().iter().for_each(|x| {
            crate::sbi::console_putchar(*x);
        });
        Ok(())
    }
}

pub fn __print(args: Arguments) {
    Stdout.write_fmt(args).unwrap();
}
