# Xây dựng OS bare-metal RISC-V 64 bằng Rust thuần + ASM

## Phân tích mã nguồn hiện tại

Dự án hiện tại có cấu trúc cơ bản đúng hướng nhưng tồn tại nhiều vấn đề cần sửa:

### Các vấn đề phát hiện

| File | Vấn đề | Mức độ |
|------|--------|--------|
| [Cargo.toml](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/Cargo.toml) | Thiếu crate-type `staticlib`, edition `2024` có thể gây lỗi feature | ⚠️ |
| [lib.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs) | `#![feature(panic_info_message, asm)]` — `asm` đã stable, `panic_info_message` đã bị remove | 🔴 |
| [lib.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs) | `print!`/`println!` macro rỗng, không output gì | 🔴 |
| [lib.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs) | `info.message().unwrap()` — API đã thay đổi trong Rust mới | 🔴 |
| [lib.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs) | [kmain()](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs#45-49) rỗng | ⚠️ |
| [Makefile](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/Makefile) | Dùng `riscv64-unknown-linux-gnu-g++` (C++) để link — không cần thiết | ⚠️ |
| [boot.S](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/asm/boot.S) | Dùng file ASM riêng, compiled bởi C++ compiler | ⚠️ |
| [virt.lds](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lds/virt.lds) | `_heap_end = _memory_end - _stack` — phép tính sai, nên là `_memory_end` | 🔴 |
| [.cargo/config.toml](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/.cargo/config.toml) | Runner command thiếu `-nographic -serial mon:stdio` đúng vị trí | ⚠️ |

## Proposed Changes

Chuyển đổi từ mô hình **C++ linker + Rust static lib** sang **pure Rust binary** sử dụng `global_asm!` cho boot code.

---

### 1. Cargo Configuration

#### [MODIFY] [Cargo.toml](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/Cargo.toml)

```diff
 [package]
 name = "learn_os_riscv"
 version = "0.1.0"
-edition = "2024"
+edition = "2021"

 [dependencies]
+
+[profile.dev]
+panic = "abort"
+
+[profile.release]
+panic = "abort"
```

- Đổi edition về `2021` (ổn định, tương thích tốt với bare-metal)
- Thêm `panic = "abort"` để không cần [eh_personality](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs#18-20)

#### [MODIFY] [config.toml](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/.cargo/config.toml)

```toml
[build]
target = "riscv64gc-unknown-none-elf"
rustflags = ['-Clink-arg=-Tsrc/lds/virt.lds']

[target.riscv64gc-unknown-none-elf]
runner = "qemu-system-riscv64 -machine virt -cpu rv64 -smp 4 -m 128M -nographic -serial mon:stdio -bios none -kernel"
```

- `runner` cho phép `cargo run` tự động gọi QEMU với binary path append vào cuối
- `make run` dùng command riêng trong Makefile — cả hai đều hoạt động

---

### 2. Boot Assembly (tích hợp vào Rust bằng `global_asm!`)

#### [NEW] [boot.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/boot.rs)

Sử dụng `core::arch::global_asm!` để nhúng assembly boot code trực tiếp trong Rust, loại bỏ phụ thuộc C++ compiler:

```rust
use core::arch::global_asm;

global_asm!(
    ".option norvc",
    ".section .text.init",
    ".globl _start",
    "_start:",
    // 1. Chỉ hart 0 chạy, các hart khác sleep
    "   csrr t0, mhartid",
    "   bnez t0, 3f",
    "   csrw satp, zero",      // Tắt paging
    // 2. Set global pointer
    ".option push",
    ".option norelax",
    "   la gp, _global_pointer",
    ".option pop",
    // 3. Xóa BSS section
    "   la a0, _bss_start",
    "   la a1, _bss_end",
    "   bgeu a0, a1, 2f",
    "1:",
    "   sd zero, 0(a0)",
    "   addi a0, a0, 8",
    "   bltu a0, a1, 1b",
    "2:",
    // 4. Set stack pointer
    "   la sp, _stack",
    // 5. Jump to Rust kmain
    "   tail kmain",
    // Hart != 0: sleep forever
    "3:",
    "   wfi",
    "   j 3b",
);
```

**Giải thích từng bước:**
1. **Hart ID check**: RISC-V multi-core, chỉ hart 0 boot, còn lại `wfi` (wait for interrupt)
2. **Tắt paging**: `satp = 0` đảm bảo chạy physical addressing
3. **Xóa BSS**: Biến toàn cục chưa khởi tạo phải = 0
4. **Set stack**: Stack pointer trỏ đến vùng nhớ sau BSS
5. **Nhảy vào Rust**: `tail kmain` — jump không return, tiết kiệm stack frame

> [!NOTE]
> So với bản gốc: đơn giản hóa boot bằng cách bỏ `mstatus`/`mepc`/`mret` flow. Ở giai đoạn đầu, ta chạy trực tiếp ở Machine mode nên không cần chuyển privilege level.

---

### 3. UART Driver

#### [NEW] [uart.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/uart.rs)

QEMU `virt` machine cung cấp UART 16550 tại địa chỉ `0x1000_0000`:

```rust
// UART base address cho QEMU virt machine
const UART_BASE: usize = 0x1000_0000;

pub fn uart_init() {
    let ptr = UART_BASE as *mut u8;
    unsafe {
        // Bật FIFO, xóa buffer
        ptr.add(2).write_volatile(0x07);
        // Bật receive interrupt
        ptr.add(1).write_volatile(0x01);
        // Set baud rate (divisor latch)
        ptr.add(3).write_volatile(0x80);  // DLAB=1
        ptr.add(0).write_volatile(0x03);  // divisor low
        ptr.add(1).write_volatile(0x00);  // divisor high
        ptr.add(3).write_volatile(0x03);  // 8-bit, no parity, 1 stop bit
    }
}

pub fn uart_putc(c: u8) {
    let ptr = UART_BASE as *mut u8;
    unsafe {
        ptr.add(0).write_volatile(c);
    }
}
```

**Giải thích:**
- UART 16550 là chip serial chuẩn, QEMU emulate nó
- `write_volatile` đảm bảo compiler không optimize bỏ memory-mapped I/O
- Init: bật FIFO, set 8N1 (8 bit data, no parity, 1 stop bit)

---

### 4. Print Infrastructure

#### [NEW] [console.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/console.rs)

Tạo `Write` trait implementation để dùng `write!`/`writeln!` → `print!`/`println!`:

```rust
use core::fmt::{self, Write};
use crate::uart;

struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            uart::uart_putc(byte);
        }
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    Console.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
```

---

### 5. Main Entry Point

#### [MODIFY] [main.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/main.rs) (rename từ [lib.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs))

```rust
#![no_std]
#![no_main]

mod boot;      // Boot assembly
mod uart;      // UART driver
mod console;   // Print macros

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("*** PANIC ***");
    println!("{}", info);
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    uart::uart_init();

    println!("Hello, RISC-V OS!");
    println!("Running on QEMU virt machine");
    println!("Hart 0, Machine mode");

    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}
```

**Flow hoàn chỉnh:**
```
QEMU boot → _start (ASM) → clear BSS → set stack → kmain (Rust) → uart_init → println!
```

---

### 6. Linker Script Fix

#### [MODIFY] [virt.lds](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lds/virt.lds)

```diff
     PROVIDE(_memory_start = ORIGIN(ram));
     PROVIDE(_stack = _bss_end + 0x80000);
     PROVIDE(_memory_end = ORIGIN(ram) + LENGTH(ram));
-    PROVIDE(_heap_start = _stack);
-    PROVIDE(_heap_end = _memory_end - _stack);
+    PROVIDE(_heap_start = _stack);
+    PROVIDE(_heap_end = _memory_end);
```

- `_heap_end` nên bằng `_memory_end`, không phải `_memory_end - _stack` (phép trừ cho ra giá trị vô nghĩa)

---

### 7. Build System

#### [MODIFY] [Makefile](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/Makefile)

```makefile
QEMU = qemu-system-riscv64
MACH = virt
CPU = rv64
CPUS = 4
MEM = 128M
TARGET = target/riscv64gc-unknown-none-elf/debug/learn_os_riscv

all:
	cargo build

run: all
	$(QEMU) -machine $(MACH) -cpu $(CPU) -smp $(CPUS) -m $(MEM) \
		-nographic -serial mon:stdio -bios none -kernel $(TARGET)

.PHONY: clean
clean:
	cargo clean
```

- Loại bỏ hoàn toàn C++ compiler
- Output trực tiếp từ `cargo build` — không cần compile ASM riêng

---

### 8. Cleanup

#### [DELETE] [boot.S](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/asm/boot.S)
#### [DELETE] [trap.S](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/asm/trap.S)

Không cần file ASM riêng vì ta dùng `global_asm!` trong Rust.

---

### 9. Custom Test Framework (chạy test trên QEMU)

Bare-metal `no_std` không có test harness mặc định. Ta dùng `custom_test_frameworks` feature để tự xây dựng hệ thống test chạy trên QEMU.

**Cách hoạt động:**
```
cargo test → build test binary → QEMU chạy binary → test runner gọi từng test
→ output kết quả qua UART → QEMU exit với exit code (pass/fail)
```

#### QEMU Exit Mechanism

QEMU `virt` machine hỗ trợ `sifive_test` device tại `0x10_0000` — ghi giá trị vào đây để QEMU thoát:
- `0x5555` = exit success (code 0)
- `0x3333` = exit failure (code 1)

#### [NEW] [qemu.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/qemu.rs)

```rust
/// QEMU virt machine sifive_test device
const QEMU_EXIT_ADDR: *mut u32 = 0x10_0000 as *mut u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0x5555,
    Failure = 0x3333,
}

pub fn exit_qemu(code: ExitCode) -> ! {
    unsafe {
        core::ptr::write_volatile(QEMU_EXIT_ADDR, code as u32);
    }
    // Nếu write không work, loop forever
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}
```

#### [NEW] [test_runner.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/test_runner.rs)

```rust
use crate::{println, qemu};

/// Trait cho mỗi test case
pub trait Testable {
    fn run(&self);
}

/// Implement cho tất cả Fn() — mỗi test function tự động có trait này
impl<T: Fn()> Testable for T {
    fn run(&self) {
        // In tên test (dùng core::any::type_name để lấy tên function)
        print!("  test {} ... ", core::any::type_name::<T>());
        self();       // Chạy test
        println!("[ok]");
    }
}

/// Test runner — được gọi bởi custom_test_frameworks
pub fn test_runner(tests: &[&dyn Testable]) {
    println!("\n=== Running {} tests ===", tests.len());
    for test in tests {
        test.run();
    }
    println!("=== All tests passed! ===\n");
    qemu::exit_qemu(qemu::ExitCode::Success);
}
```

#### [MODIFY] [main.rs](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/main.rs) — thêm test framework

```rust
#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(test_runner::test_runner))]
#![cfg_attr(test, reexport_test_harness_name = "test_main")]

mod boot;
mod uart;
mod console;
mod qemu;
#[cfg(test)]
mod test_runner;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("*** PANIC ***");
    println!("{}", info);
    #[cfg(test)]
    qemu::exit_qemu(qemu::ExitCode::Failure);
    #[cfg(not(test))]
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    uart::uart_init();

    // Nếu đang test → chạy test_main() thay vì main logic
    #[cfg(test)]
    test_main();

    println!("Hello, RISC-V OS!");
    println!("Running on QEMU virt machine");
    println!("Hart 0, Machine mode");

    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

// ============ TEST CASES ============

/// Test 1: UART putc hoạt động
#[cfg(test)]
#[test_case]
fn test_uart_output() {
    // Nếu đến đây mà không panic → UART hoạt động
    uart::uart_putc(b'X');
}

/// Test 2: println! macro hoạt động
#[cfg(test)]
#[test_case]
fn test_println() {
    println!("test output: println works!");
}

/// Test 3: println! với format arguments
#[cfg(test)]
#[test_case]
fn test_println_format() {
    println!("formatted: {} + {} = {}", 1, 2, 1 + 2);
}

/// Test 4: Nhiều println liên tiếp không crash
#[cfg(test)]
#[test_case]
fn test_println_many() {
    for i in 0..100 {
        println!("line {}", i);
    }
}

/// Test 5: Boot — nếu test này chạy được = boot thành công
#[cfg(test)]
#[test_case]
fn test_boot_success() {
    // Test đơn giản: nếu code đến được đây, boot sequence đã OK
    assert_eq!(1 + 1, 2);
}

/// Test 6: Stack hoạt động (đệ quy)
#[cfg(test)]
#[test_case]
fn test_stack_works() {
    fn fibonacci(n: u64) -> u64 {
        if n <= 1 { return n; }
        fibonacci(n - 1) + fibonacci(n - 2)
    }
    assert_eq!(fibonacci(10), 55);
}

/// Test 7: BSS section đã được xóa
#[cfg(test)]
#[test_case]
fn test_bss_zeroed() {
    static BSS_VAR: core::sync::atomic::AtomicU64 = 
        core::sync::atomic::AtomicU64::new(0);
    assert_eq!(
        BSS_VAR.load(core::sync::atomic::Ordering::Relaxed), 
        0
    );
}
```

**Giải thích `#[cfg_attr(test, ...)]`:**
- `custom_test_frameworks` — cho phép ta tự định nghĩa test runner thay vì dùng `std::test`
- `test_runner(...)` — function sẽ được gọi với danh sách test
- `reexport_test_harness_name` — rename [main](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/src/lib.rs#45-49) test thành `test_main` để ta gọi thủ công sau khi init UART
- `#[test_case]` — đánh dấu function là test (thay vì `#[test]`)

#### [MODIFY] [Makefile](file:///Users/doveun/Documents/Project/Rust/Rust_learn/learn_os_riscv/Makefile) — thêm target `test`

Thêm vào Makefile:

```makefile
# Tìm binary test (cargo test --no-run output path)
TEST_BIN = $(shell cargo test --no-run --message-format=json 2>/dev/null \
	| python3 -c "import sys,json; [print(l['executable']) for l in (json.loads(line) for line in sys.stdin) if l.get('executable')]" \
	| head -1)

test:
	cargo test --no-run
	$(QEMU) -machine $(MACH) -cpu $(CPU) -smp $(CPUS) -m $(MEM) \
		-nographic -serial mon:stdio -bios none -kernel $(TEST_BIN) \
		-device sifive_test
```

> [!IMPORTANT]
> `-device sifive_test` là bắt buộc — nó cho phép guest ghi vào `0x10_0000` để exit QEMU với exit code.

## Verification Plan

### Automated Tests

```bash
# 1. Build
cargo build

# 2. Kiểm tra binary format
file target/riscv64gc-unknown-none-elf/debug/learn_os_riscv
# Expected: ELF 64-bit LSB executable, UCB RISC-V

# 3a. Run OS bằng cargo run
cargo run
# 3b. Hoặc dùng make run (kết quả giống nhau)
make run
# Expected output: "Hello, RISC-V OS!"
# Ctrl+A rồi X để thoát

# 4. Run tests trên QEMU
make test
# Expected output:
#   === Running 7 tests ===
#     test test_uart_output ... [ok]
#     test test_println ... [ok]
#     test test_println_format ... [ok]
#     test test_println_many ... [ok]
#     test test_boot_success ... [ok]
#     test test_stack_works ... [ok]
#     test test_bss_zeroed ... [ok]
#   === All tests passed! ===
# QEMU exits automatically with code 0
```

### Manual Verification
- `make run` → thấy "Hello, RISC-V OS!"
- `make test` → thấy tất cả tests pass, QEMU tự thoát

Để kết thúc QEMU khi đang chạy ở chế độ -nographic (như trong cấu hình hiện tại), bạn sử dụng tổ hợp phím sau:

Nhấn Ctrl + a (thả ra)
Sau đó nhấn phím x
Giải thích:

Trong chế độ -nographic, QEMU sử dụng terminal của bạn làm màn hình console. Phím Ctrl + c thường sẽ bị gửi vào hệ điều hành đang chạy trong QEMU thay vì tắt QEMU.
Ctrl + a là phím điều khiển (escape key) của QEMU. Sau khi nhấn nó, QEMU sẽ chờ lệnh tiếp theo. Phím x viết tắt của "exit".
Một số lệnh hữu ích khác sau khi nhấn Ctrl + a:

Ctrl + a rồi nhấn c: Chuyển sang Monitor mode của QEMU (để debug, kiểm tra register, memory...). Nhấn lại lần nữa để quay lại console.
Ctrl + a rồi nhấn h: Xem danh sách tất cả các lệnh điều khiển.