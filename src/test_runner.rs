// test_runner.rs — Custom test framework cho bare-metal
//
// Trong môi trường no_std, không có std::test. Ta dùng
// #![feature(custom_test_frameworks)] để tự định nghĩa
// cách test được thu thập và chạy.

use crate::qemu;

/// Trait cho mỗi test case
pub trait Testable {
    fn run(&self);
}

/// Implement Testable cho tất cả Fn()
/// → mỗi test function tự động có trait này
impl<T: Fn()> Testable for T {
    fn run(&self) {
        // In tên test (lấy từ type name của closure/function)
        crate::console::_print(format_args!("  test {} ... ", core::any::type_name::<T>()));
        self();            // Chạy test — nếu panic → test fail
        crate::console::_print(format_args!("[ok]\n"));
    }
}

/// Test runner — được gọi bởi custom_test_frameworks
///
/// Nhận danh sách tất cả #[test_case] functions,
/// chạy từng cái, rồi exit QEMU với success code.
pub fn test_runner(tests: &[&dyn Testable]) {
    crate::console::_print(format_args!("\n=== Running {} tests ===\n", tests.len()));
    for test in tests {
        test.run();
    }
    crate::console::_print(format_args!("=== All tests passed! ===\n\n"));
    qemu::exit_qemu(qemu::ExitCode::Success);
}
