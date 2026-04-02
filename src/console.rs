// console.rs — Print infrastructure
//
// Implement core::fmt::Write cho UART để dùng được format strings.
// Cung cấp print! và println! macros giống std library.

use core::fmt::{self, Write};
use crate::uart;

/// Console struct — implement Write trait để format output qua UART
struct Console;

impl Write for Console {
    /// Ghi từng byte của string ra UART
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            uart::uart_putc(byte);
        }
        Ok(())
    }
}

/// Hàm internal — được gọi bởi print!/println! macros
pub fn _print(args: fmt::Arguments) {
    Console.write_fmt(args).unwrap();
}

/// In text ra serial console (không xuống dòng)
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::console::_print(format_args!($($arg)*)) };
}

/// In text ra serial console (có xuống dòng)
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { $crate::print!("{}\n", format_args!($($arg)*)) };
}
