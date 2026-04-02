// uart.rs — UART 16550 driver cho QEMU virt machine
//
// QEMU virt machine emulate UART 16550 tại địa chỉ 0x1000_0000.
// Đây là chip serial chuẩn công nghiệp, được dùng rộng rãi trong PC và embedded.
//
// Registers (offset từ base address):
//   0: THR/RBR — Transmit/Receive Buffer (đọc/ghi data)
//   1: IER     — Interrupt Enable Register
//   2: FCR/IIR — FIFO Control / Interrupt Identification
//   3: LCR     — Line Control Register (format data: bits, parity, stop)
//   4: MCR     — Modem Control Register
//   5: LSR     — Line Status Register (trạng thái transmit/receive)

/// Địa chỉ base của UART trên QEMU virt machine
const UART_BASE: usize = 0x1000_0000;

/// Khởi tạo UART 16550
///
/// Cấu hình: 8 data bits, no parity, 1 stop bit (8N1) — chuẩn phổ biến nhất
pub fn uart_init() {
    let ptr = UART_BASE as *mut u8;
    unsafe {
        // 1. Bật FIFO và xóa buffer (FCR register, offset 2)
        //    Bit 0: Enable FIFO
        //    Bit 1: Clear receive FIFO
        //    Bit 2: Clear transmit FIFO
        ptr.add(2).write_volatile(0x07);   // 0b0000_0111

        // 2. Bật receive interrupt (IER register, offset 1)
        //    Bit 0: Receive data available interrupt
        ptr.add(1).write_volatile(0x01);

        // 3. Set baud rate — dùng Divisor Latch
        //    Bước 3a: Bật DLAB (Divisor Latch Access Bit) trong LCR
        ptr.add(3).write_volatile(0x80);   // LCR: DLAB=1

        //    Bước 3b: Set divisor = 3 (38400 baud với 1.8432 MHz clock)
        //    Divisor Low byte (offset 0 khi DLAB=1)
        ptr.add(0).write_volatile(0x03);
        //    Divisor High byte (offset 1 khi DLAB=1)
        ptr.add(1).write_volatile(0x00);

        // 4. Tắt DLAB, set format: 8 data bits, no parity, 1 stop bit
        //    LCR: Bit 0-1 = 11 (8 bits), Bit 2 = 0 (1 stop), Bit 3 = 0 (no parity)
        ptr.add(3).write_volatile(0x03);   // 0b0000_0011
    }
}

/// Gửi 1 byte qua UART (blocking)
///
/// Ghi trực tiếp vào THR (Transmit Holding Register, offset 0).
/// QEMU không cần check LSR (Line Status) vì transmit luôn sẵn sàng.
pub fn uart_putc(c: u8) {
    let ptr = UART_BASE as *mut u8;
    unsafe {
        // write_volatile đảm bảo compiler không optimize bỏ
        // vì đây là Memory-Mapped I/O (MMIO), không phải RAM thường
        ptr.add(0).write_volatile(c);
    }
}
