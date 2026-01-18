use x86_64::instructions::port::Port;

// Constants for PIT (Programmable Interval Timer)
const PIT_CHANNEL0_DATA: u16 = 0x40; // Data port for channel 0
const PIT_COMMAND_PORT: u16 = 0x43; // Command port

// PIT base frequency is 1.193182 MHz
const PIT_FREQUENCY: u32 = 1_193_182;

// Frequency of the timer interrupt in Hz
pub const TIMER_FREQUENCY_HZ: u32 = 100;

// Calculate divisor needed to get desired frequency
// Cast to u16 after division to avoid type mismatch
const PIT_DIVISOR: u16 = (PIT_FREQUENCY / TIMER_FREQUENCY_HZ) as u16; // ~11932

/// Global tick counter - incremented by the timer interrupt handler
pub static mut TICK_COUNT: u64 = 0;

/// Initialize the Programmable Interval Timer (PIT) for periodic interrupts at 100Hz
pub fn init_pit() {
    unsafe {
        // Command byte: 0x36 = 00110110b
        // Bits 7-6: 00 = Channel 0
        // Bits 5-4: 11 = Access mode lo/hi byte
        // Bits 3-1: 011 = Mode 3 (square wave generator)
        // Bit 0: 0 = Binary mode (not BCD)
        let mut command_port: Port<u8> = Port::new(PIT_COMMAND_PORT);
        command_port.write(0x36);

        // Send the divisor value to channel 0 (low byte first, then high byte)
        let mut data_port: Port<u8> = Port::new(PIT_CHANNEL0_DATA);

        // Low byte of divisor
        data_port.write((PIT_DIVISOR & 0xFF) as u8);
        // High byte of divisor
        data_port.write((PIT_DIVISOR >> 8) as u8);
    }

    crate::serial_println!("PIT initialized at {}Hz (divisor: {})", TIMER_FREQUENCY_HZ, PIT_DIVISOR);
}

/// Called by the timer interrupt handler to increment the tick count
#[inline]
pub fn tick() {
    unsafe {
        TICK_COUNT = TICK_COUNT.wrapping_add(1);
    }
}

/// Get current tick count (number of timer interrupts since boot)
pub fn get_ticks() -> u64 {
    unsafe { TICK_COUNT }
}

/// Get elapsed seconds since boot
#[allow(dead_code)]
pub fn get_seconds() -> u64 {
    get_ticks() / TIMER_FREQUENCY_HZ as u64
}
