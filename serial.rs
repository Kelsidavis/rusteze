// Serial port driver for COM1

use core::fmt;
use spin::Mutex;

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        SerialPort { port }
    }

    // Check if the serial port is ready to send a byte
    #[inline]
    fn is_ready(&self) -> bool {
        unsafe {
            core::ptr::read_volatile((self.port + 5) as *const u8) & 0x20 != 0
        }
    }

    pub fn send_byte(&mut self, byte: u8) {
        while !self.is_ready() {}
        
        unsafe {
            core::ptr::write_volatile((self.port as *const u8).add(0), byte);
        }
    }

    pub fn send_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.send_byte(byte),
                _ => self.send_byte(0xfe), // placeholder for non-printable
            }
        }
    }

    pub fn send_char(&mut self, c: char) {
        let bytes = [c as u8];
        self.send_string(&String::from_utf8_lossy(&bytes));
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.send_string(s);
        Ok(())
    }
}

// Initialize the serial port at COM1 (0x3F8)
pub fn init_serial_port() -> Option<Mutex<SerialPort>> {
    let mut serial = Some(SerialPort::new(0x3F8));
    
    // Disable interrupts and set DLAB to 1
    unsafe {
        core::ptr::write_volatile((serial.as_ref().unwrap().port + 2) as *mut u8, 0);
        
        // Set baud rate divisor (115200 / 9600 = 12)
        let div = 115_200 / 9600;
        core::ptr::write_volatile(
            (serial.as_ref().unwrap().port + 3) as *mut u8,
            ((div >> 8) & 0xFF),
        );
        
        // Set low byte of divisor
        core::ptr::write_volatile((serial.as_ref().unwrap().port + 4) as *mut u8, div & 0xFF);
    }

    // Configure line: 8 data bits, no parity, 1 stop bit (0x03)
    unsafe {
        core::ptr::write_volatile(
            (serial.as_ref().unwrap().port + 3) as *const u8,
            0x03 | 0b11 << 6, // Set DLAB = 0 and configure line
        );
        
        // Enable FIFO buffers with trigger level at 1 byte
        unsafe {
            core::ptr::write_volatile((serial.as_ref().unwrap().port + 2) as *mut u8, 0xC7);
            
            // Clear transmit buffer (bit 3)
            let mut temp = core::ptr::read_volatile(
                (serial.as_ref().unwrap().port + 1) as *const u8,
            );
            temp |= 0x4;
            core::ptr::write_volatile((serial.as_ref().unwrap().port + 1) as *mut u8, temp);
            
            // Clear receive buffer
            let mut temp = core::ptr::read_volatile(
                (serial.as_ref().unwrap().port + 2) as *const u8,
            );
            temp |= 0x4;
            core::ptr::write_volatile((serial.as_ref().unwrap().port + 1) as *mut u8, temp);
        }
    }

    // Enable interrupts
    unsafe {
        let mut val = core::ptr::read_volatile(
            (serial.as_ref().unwrap().port + 3) as *const u8,
        );
        val |= 0x2;
        core::ptr::write_volatile((serial.as_ref().unwrap().port + 3) as *mut u8, val);
    }

    Some(Mutex::new(serial.unwrap()))
}

// Global static instance of the serial port (initialized by init_serial_port)
lazy_static::lazy_static! {
    pub static ref SERIAL_INIT: Mutex<Option<SerialPort>> = match init_serial_port() {
        Some(port) => Mutex::new(Some(port)),
        None => panic!("Failed to initialize COM1 serial port"),
    };
}
