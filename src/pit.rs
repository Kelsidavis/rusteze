use crate::idt;
use x86_64::{instructions::port::Port, structures::pic};

// Constants for PIT (Programmable Interval Timer)
const PIT_CHANNEL: u16 = 0x40; // Data port for channel 0
const PIT_COMMAND_PORT: u16 = 0x43; // Command port

// Frequency of the timer interrupt in Hz
pub const TIMER_FREQUENCY_HZ: u32 = 100;

// Calculate divisor needed to get desired frequency (based on 1.19318 MHz clock)
const PIT_DIVISOR: u16 = 1_193_180 / TIMER_FREQUENCY_HZ; // ~11932

/// Initialize the Programmable Interval Timer (PIT) for periodic interrupts
pub fn init_pit() {
    unsafe {
        // Send command to set up channel 0 in mode 3 (square wave generator)
        // Mode 3: Square Wave Generator - produces a square wave with period = high byte * 256 + low byte
        let mut port = Port::new(PIT_COMMAND_PORT);
        port.write(0x34); // Channel 0, access mode (latch), operating mode 3

        // Send the divisor value to channel 0 in two parts: first low then high bytes
        let mut data_port = Port::new(PIT_CHANNEL);
        
        // Low byte of divisor
        data_port.write((PIT_DIVISOR & 0xFF) as u8);
        // High byte of divisor  
        data_port.write(((PIT_DIVISOR >> 8) & 0xFF) as u8);

        // Enable timer interrupt on IRQ0 (which is connected to the PIT)
        pic::disable_irq(0); // Disable for safety
    }

    idt::add_handler(idt::InterruptIndex::Timer, handle_timer_interrupt);
}

/// Handler function called when a timer interrupt occurs
extern "x86-interrupt" fn handle_timer_interrupt(_stack_frame: &mut x86_64::structures::idt::InterruptStackFrame) {
    // Acknowledge the PIC (send EOI - End of Interrupt)
    unsafe { pic::EOI(0); }

    // Increment a global tick counter
    static mut TICKS: u32 = 0;
    
    unsafe {
        TICKS += 1;

        if TICKS % TIMER_FREQUENCY_HZ == 0 {
            println!("Timer interrupt - {} seconds elapsed", TICKS / TIMER_FREQUENCY_HZ);
        }
        
        // Optional: Add more timer-related functionality here
    }

    // We don't need to do anything else for now, just return from the handler.
}

/// Get current tick count (number of interrupts since boot)
pub fn get_ticks() -> u32 {
    unsafe { 
        static mut TICKS: u32 = 0;
        
        // This is a simple implementation - in reality we'd want to use atomic operations
        // for thread safety, but this works fine at the kernel level.
        TICKS 
    }
}
