// qemu.rs — QEMU exit mechanism
//
// QEMU virt machine hỗ trợ "sifive_test" device.
// Khi thêm `-device sifive_test` vào QEMU command line,
// guest có thể ghi vào địa chỉ 0x10_0000 để exit QEMU
// với exit code tương ứng.
//
// Dùng cho test framework: test pass → exit 0, test fail → exit 1.

/// Địa chỉ sifive_test device trên QEMU virt machine
const QEMU_EXIT_ADDR: *mut u32 = 0x10_0000 as *mut u32;

/// Exit code cho QEMU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// QEMU thoát với code 0 (success)
    Success = 0x5555,
    /// QEMU thoát với code 1 (failure)  
    Failure = 0x3333,
}

/// Thoát QEMU với exit code chỉ định
///
/// Ghi giá trị vào sifive_test device address.
/// Nếu device không có (chạy trên hardware thật), loop forever.
pub fn exit_qemu(code: ExitCode) -> ! {
    unsafe {
        core::ptr::write_volatile(QEMU_EXIT_ADDR, code as u32);
    }
    // Fallback nếu write không work (hardware thật hoặc device chưa enable)
    loop {
        unsafe { core::arch::asm!("wfi"); }
    }
}
