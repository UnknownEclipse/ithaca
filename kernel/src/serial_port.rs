use core::{arch::asm, fmt, hint};

use bitflags::bitflags;

#[derive(Debug)]
pub struct SpinWriter {
    port: SerialPort,
}

impl SpinWriter {
    pub fn new(serial_port: SerialPort) -> Self {
        Self { port: serial_port }
    }
}

impl fmt::Write for SpinWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.as_bytes() {
            while let Err(SendError::Full) = self.port.send(*byte) {
                hint::spin_loop();
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    unsafe fn init(&mut self) {
        self.write(1, 0);

        // Configure DLAB, set BAUD to 0x3 (38400)
        self.write(3, 0x80);
        self.write(0, 0x3);
        self.write(1, 0);

        // Configure line control to 8n1
        self.write(3, 0x3);

        self.write(2, 0xc7);
        self.write(4, 0xb);
        self.write(1, 0x1);
    }

    pub unsafe fn com1() -> SerialPort {
        let mut port = SerialPort { port: 0x3f8 };
        port.init();
        port
    }

    pub fn send(&mut self, byte: u8) -> Result<(), SendError> {
        let line_status = self.line_status();
        if line_status.contains(LineStatus::TRANSMIT_BUFFER_EMPTY) {
            unsafe { self.write(0, byte) };
            Ok(())
        } else {
            Err(SendError::Full)
        }
    }

    pub fn recv(&mut self) -> Result<u8, RecvError> {
        let line_status = self.line_status();
        if line_status.contains(LineStatus::DATA_READY) {
            Ok(unsafe { self.read(0) })
        } else {
            Err(RecvError::Empty)
        }
    }

    fn line_status(&self) -> LineStatus {
        unsafe { LineStatus::from_bits_retain(self.read(5)) }
    }

    unsafe fn write(&self, register: u8, value: u8) {
        out8(self.port + u16::from(register), value);
    }

    unsafe fn read(&self, register: u8) -> u8 {
        in8(self.port + u16::from(register))
    }
}

#[derive(Debug)]
pub enum SendError {
    Full,
}

#[derive(Debug)]
pub enum RecvError {
    Empty,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct InterruptFlags: u8 {
        const DATA_AVAILABLE = 1;
        const TRANSMITTER_EMPTY = 1 << 1;
        const ERROR = 1 << 2;
        const STATUS_CHANGE = 1 << 3;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct LineStatus: u8 {
        const DATA_READY = 1;
        const OVERRUN_ERROR = 1 << 1;
        const PARITY_ERROR = 1 << 2;
        const FRAMING_ERROR = 1 << 3;
        const BREAK_INDICATOR = 1 << 4;
        const TRANSMIT_BUFFER_EMPTY = 0x20;
        const TRANSMITTER_EMPTY = 1 << 6;
        const IMPENDING_ERROR = 1 << 7;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct LineControl: u8 {
        const DLAB = 1 << 7;
    }
}

#[inline]
unsafe fn out8(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value);
}

#[inline]
unsafe fn in8(port: u16) -> u8 {
    let value;
    asm!("in al, dx", in("dx") port, out("al") value);
    value
}
