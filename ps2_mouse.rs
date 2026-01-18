// src/ps2_mouse.rs

use crate::io::{inb, outb};
use x86_64::instructions::interrupts;

/// PS/2 mouse port addresses
const MOUSE_DATA_PORT: u16 = 0x60;
const MOUSE_COMMAND_PORT: u16 = 0x64;

// Mouse commands
const CMD_READ_STATUS: u8 = 0xE9;      // Read status register
const CMD_WRITE_CONFIG: u8 = 0x60;     // Write configuration byte
const CMD_ENABLE_MOUSE: u8 = 0xA8;    // Enable mouse data reporting
const CMD_DISABLE_MOUSE: u8 = 0xA7;   // Disable mouse data reporting
const CMD_SET_SAMPLE_RATE: u8 = 0xF3; // Set sample rate

// Status register bits (from PS/2 controller)
const STATUS_DATA_READY: u8 = 1 << 0;     // Data ready in output buffer
const STATUS_COMMAND_BYTE: u8 = 1 << 2;   // Command byte received, not data
const STATUS_MOUSE_INTERRUPT: u8 = 1 << 5; // Mouse interrupt occurred

/// PS/2 mouse packet structure (3 bytes for standard mice)
#[derive(Debug)]
pub struct MouseEvent {
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    pub x_movement: i8,
    pub y_movement: i8,
}

impl MouseEvent {
    /// Creates a new empty event
    fn new() -> Self {
        MouseEvent {
            left_button: false,
            right_button: false,
            middle_button: false,
            x_movement: 0,
            y_movement: 0,
        }
    }

    /// Updates the mouse state from raw bytes (3-byte packet)
    pub fn update(&mut self, data: &[u8]) {
        if data.len() < 3 {
            return;
        }

        // First byte - status bits
        let first_byte = data[0];
        
        // Button states (bits 1-2 for left/right/middle buttons)
        self.left_button = (first_byte & 0x01) != 0;
        self.right_button = (first_byte & 0x02) != 0;
        self.middle_button = (first_byte & 0x04) != 0;

        // Movement data
        let x_movement = if first_byte & 0x80 == 0 {
            i8::from(data[1])
        } else {
            -((!data[1] + 1) as i8)
        };

        self.x_movement = x_movement;
        
        let y_movement = if first_byte & 0x20 == 0 {
            i8::from(data[2])
        } else {
            -( (!data[2] + 1) as i8 )
        };
        
        self.y_movement = y_movement;
    }
}

/// PS/2 Mouse driver
pub struct Ps2MouseDriver {
    buffer: [u8; 3],
    index: usize,
    enabled: bool,
}

impl Ps2MouseDriver {
    /// Creates a new mouse driver instance
    pub const fn new() -> Self {
        Ps2MouseDriver {
            buffer: [0, 0, 0],
            index: 0,
            enabled: false,
        }
    }

    /// Initializes the PS/2 mouse controller and enables data reporting
    pub unsafe fn init(&mut self) {
        // Wait for input buffer to be empty (if needed)
        while inb(MOUSE_COMMAND_PORT) & STATUS_DATA_READY != 0 {}
        
        // Send command to read configuration byte
        outb(MOUSE_COMMAND_PORT, CMD_READ_STATUS);
        
        let config = inb(MOUSE_DATA_PORT); 
        
        // Enable mouse data reporting by setting bit 4 (bit 5 is for aux port)
        let new_config = config | 0x10;
        
        // Send command to write configuration byte
        outb(MOUSE_COMMAND_PORT, CMD_WRITE_CONFIG);
        while inb(MOUSE_COMMAND_PORT) & STATUS_DATA_READY != 0 {}
        outb(MOUSE_DATA_PORT, new_config);

        // Enable data reporting from mouse
        while inb(MOUSE_COMMAND_PORT) & STATUS_DATA_READY != 0 {}
        outb(MOUSE_COMMAND_PORT, CMD_ENABLE_MOUSE);
        
        self.enabled = true;
    }

    /// Handles incoming PS/2 mouse interrupt (IRQ12)
    pub fn handle_interrupt(&mut self) {
        if !self.enabled {
            return;
        }
        
        let data = unsafe { inb(MOUSE_DATA_PORT) };
        
        // Store byte and update index
        self.buffer[self.index] = data;
        self.index += 1;

        // If we have a complete packet (3 bytes), process it
        if self.index == 3 {
            self.process_packet();
            self.reset_buffer();
        }
    }

    /// Processes the collected mouse packet
    fn process_packet(&mut self) {
        let mut event = MouseEvent::new();
        event.update(&self.buffer);
        
        // For now, just print debug info about movement and button states
        if event.x_movement != 0 || event.y_movement != 0 {
            println!("Mouse moved: x={}, y={}", 
                     event.x_movement as i32,
                     event.y_movement as i32);
            
            let mut msg = String::from("Mouse move ");
            if event.left_button { msg.push_str("(L)"); }
            if event.right_button { msg.push_str("(R)"); }
            if event.middle_button { msg.push_str("(M)"); }
            println!("{}", msg);
        } else {
            // Only print button events
            let mut msg = String::from("Mouse ");
            
            if event.left_button && !event.right_button && !event.middle_button {
                msg.push_str("left click");
            } else if !event.left_button && event.right_button && !event.middle_button {
                msg.push_str("right click");
            } else if !event.left_button && !event.right_button && event.middle_button {
                msg.push_str("middle click");
            }
            
            println!("{}", msg);
        }

    }

    /// Resets the buffer after processing a complete packet
    fn reset_buffer(&mut self) {
        self.index = 0;
    }

    /// Returns true if mouse is enabled and ready to receive data
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// Global instance of PS/2 Mouse driver (protected by spinlock)
pub static mut MOUSE_DRIVER: crate::spin::SpinLock<Ps2MouseDriver> = 
    unsafe { crate::spin::SpinLock::new(Ps2MouseDriver::new()) };

/// Interrupt handler for mouse IRQ12
#[x86_64::irq_handler]
unsafe extern "C" fn ps2_mouse_interrupt(_stack_frame: &mut x86_64::structures::idt::InterruptStackFrame) {
    // Acquire the lock to safely access shared state
    let mut driver = MOUSE_DRIVER.lock();
    
    // Handle the interrupt (process data)
    driver.handle_interrupt();

    // Send EOI (End of Interrupt) signal to PIC
    unsafe { 
        x86_64::instructions::interrupts::disable();  // Disable interrupts temporarily
        outb(0x20, 0x20);   // Send EOI for master PIC
        outb(0xA0, 0x20);   // Send EOI for slave PIC  
    }
}

// Implementation of the `init_mouse` function that will be called from lib.rs
pub fn init_mouse() {
    unsafe {
        MOUSE_DRIVER.lock().init();
        
        // Set up interrupt handler in IDT (IRQ12 = vector 36)
        idt::set_handler(36, ps2_mouse_interrupt);
    }
}
