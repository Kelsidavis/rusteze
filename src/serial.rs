// Serial port driver for COM1
use core::fmt;
use spin::Mutex;
use lazy_static::lazy_static;

const COM1: u16 = 0x3F8;

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        SerialPort { port }
    }

    /// Initialize the serial port with standard settings
    /// 9600 baud, 8N1 (8 data bits, no parity, 1 stop bit)
    pub fn init(&self) {
        unsafe {
            // Disable interrupts
            Self::outb(self.port + 1, 0x00);

            // Enable DLAB (set baud rate divisor)
            Self::outb(self.port + 3, 0x80);

            // Set divisor to 12 (lo byte) 115200 / 9600 = 12
            Self::outb(self.port + 0, 0x0C);
            Self::outb(self.port + 1, 0x00); // hi byte

            // 8 bits, no parity, one stop bit (8N1), disable DLAB
            Self::outb(self.port + 3, 0x03);

            // Enable FIFO, clear them, with 14-byte threshold
            Self::outb(self.port + 2, 0xC7);

            // IRQs enabled, RTS/DSR set
            Self::outb(self.port + 4, 0x0B);
        }
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe { Self::inb(self.port + 5) & 0x20 != 0 }
    }

    pub fn write_byte(&self, byte: u8) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        unsafe {
            Self::outb(self.port, byte);
        }
    }

    #[inline]
    unsafe fn outb(port: u16, value: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }

    #[inline]
    unsafe fn inb(port: u16) -> u8 {
        let value: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
        value
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let serial = SerialPort::new(COM1);
        serial.init();
        Mutex::new(serial)
    };
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_serial_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)))
}

#[doc(hidden)]
pub fn _serial_print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).unwrap();
}
