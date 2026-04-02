// main.rs — Kernel entry point
//
// Flow: _start (ASM) → kmain (Rust) → uart_init → println!
//
// Attributes:
//   #![no_std]  — Không dùng standard library (OS chưa có!)
//   #![no_main] — Không dùng Rust main(), ta tự định nghĩa entry point

#![no_std]
#![no_main]

// Test framework attributes (chỉ bật khi cargo test)
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(test_runner::test_runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]

// === Modules ===
mod boot;       // Boot assembly (global_asm!)
mod uart;       // UART 16550 driver
mod console;    // print!/println! macros
mod qemu;       // QEMU exit mechanism

#[cfg(test)]
mod test_runner; // Custom test framework

use core::panic::PanicInfo;

// =============================================================
// Panic Handler
//   Bắt buộc trong no_std — được gọi khi panic!() hoặc assert fail
// =============================================================
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("*** KERNEL PANIC ***");
    println!("{}", info);

    // Khi test: exit QEMU với failure code
    #[cfg(test)]
    qemu::exit_qemu(qemu::ExitCode::Failure);

    // Khi chạy bình thường: loop forever
    #[cfg(not(test))]
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

// =============================================================
// Kernel Main — Entry point từ boot assembly
//   #[no_mangle]  — giữ tên hàm "kmain" cho ASM gọi được
//   extern "C"    — dùng C calling convention
//   -> !          — never returns
// =============================================================
#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    // Bước 1: Khởi tạo UART để có thể in ra serial console
    uart::uart_init();

    // Khi chạy test → gọi test_main() (generated bởi custom_test_frameworks)
    #[cfg(test)]
    test_main();

    // Bước 2: In thông tin boot
    println!("==============================");
    println!("  Hello, RISC-V OS!");
    println!("  Running on QEMU virt machine");
    println!("  Hart 0, Machine mode");
    println!("  Rust bare-metal kernel");
    println!("==============================");

    // Bước 3: Kernel loop — chờ interrupt
    // Trong tương lai sẽ thêm scheduler, syscall handler, etc.
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}

// =============================================================
// TEST CASES
//   Chạy bằng: make test
//   Dùng #[test_case] thay vì #[test] (custom_test_frameworks)
// =============================================================

/// Test 1: UART putc hoạt động — nếu không panic = pass
#[cfg(test)]
#[test_case]
fn test_uart_output() {
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
    for i in 0..5 {
        println!("line {}", i);
    }
}

/// Test 5: Boot thành công — nếu code đến được đây = boot OK
#[cfg(test)]
#[test_case]
fn test_boot_success() {
    assert_eq!(1 + 1, 2);
}

/// Test 6: Stack hoạt động — đệ quy fibonacci
#[cfg(test)]
#[test_case]
fn test_stack_works() {
    fn fibonacci(n: u64) -> u64 {
        if n <= 1 { return n; }
        fibonacci(n - 1) + fibonacci(n - 2)
    }
    assert_eq!(fibonacci(10), 55);
}

/// Test 7: BSS section đã được xóa (biến static = 0)
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
