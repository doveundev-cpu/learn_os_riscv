// boot.rs — Boot assembly tích hợp trong Rust bằng global_asm!
//
// Flow: QEMU load kernel → _start → clear BSS → set stack → jump to kmain
//
// Chạy ở Machine mode (M-mode), privilege level cao nhất của RISC-V.

use core::arch::global_asm;

global_asm!(
    // Không dùng compressed instructions (16-bit) — đảm bảo alignment
    ".option norvc",

    // Đặt code vào section .text.init — linker script sẽ đặt nó đầu tiên
    ".section .text.init",
    ".globl _start",

    "_start:",
    //==========================================================
    // Bước 1: Kiểm tra Hart ID
    //   RISC-V có nhiều core (hart). Chỉ hart 0 boot kernel,
    //   các hart khác nhảy đến label 3 và sleep (wfi).
    //==========================================================
    "   csrr t0, mhartid",        // t0 = hardware thread id
    "   bnez t0, 3f",              // if t0 != 0 → jump to label 3 (sleep)

    //==========================================================
    // Bước 2: Tắt paging (virtual memory)
    //   satp = 0 → dùng physical addressing trực tiếp
    //==========================================================
    "   csrw satp, zero",

    //==========================================================
    // Bước 3: Set Global Pointer (gp)
    //   RISC-V dùng gp để truy cập nhanh các biến global
    //   .option norelax ngăn linker tối ưu hóa lệnh này
    //==========================================================
    ".option push",
    ".option norelax",
    "   la gp, _global_pointer",   // gp = địa chỉ _global_pointer từ linker script
    ".option pop",

    //==========================================================
    // Bước 4: Xóa BSS section (biến toàn cục chưa khởi tạo = 0)
    //   Loop từ _bss_start đến _bss_end, ghi 0 mỗi 8 bytes
    //==========================================================
    "   la a0, _bss_start",        // a0 = start address
    "   la a1, _bss_end",          // a1 = end address
    "   bgeu a0, a1, 2f",          // if start >= end → skip (BSS rỗng)
    "1:",
    "   sd zero, 0(a0)",           // *a0 = 0 (store doubleword = 8 bytes)
    "   addi a0, a0, 8",           // a0 += 8
    "   bltu a0, a1, 1b",          // if a0 < a1 → loop lại label 1

    //==========================================================
    // Bước 5: Set Stack Pointer
    //   _stack được định nghĩa trong linker script = _bss_end + 512KB
    //   Stack RISC-V mọc xuống (grow downward)
    //==========================================================
    "2:",
    "   la sp, _stack",            // sp = đỉnh stack

    //==========================================================
    // Bước 6: Nhảy vào Rust code (kmain)
    //   tail = jump không lưu return address (tail call)
    //   kmain() là hàm Rust #[no_mangle] extern "C"
    //==========================================================
    "   tail kmain",

    //==========================================================
    // Hart != 0: Sleep forever
    //   wfi = Wait For Interrupt — CPU ngủ, tiết kiệm năng lượng
    //==========================================================
    "3:",
    "   wfi",
    "   j 3b",                     // loop lại label 3 (phòng spurious wakeup)
);
