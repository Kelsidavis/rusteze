#![no_std]
#![no_main]

extern crate lazy_static;

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod vga;
mod serial; // Added for COM1 driver

// Initialize on module load (via lazy_static)
lazy_static::lazy_static! {
    static ref SERIAL_INIT: () = {
        use crate::serial::*;
        
        if let Some(serial_port) = &SERIAL_INIT.try_lock() {
            writeln!(serial_port, "Serial port initialized successfully").unwrap();
            
            println!("Testing dual output: VGA + COM1");
        } else {
            panic!("Failed to initialize serial port!");
        }
    };
}

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    vga::WRITER.lock().clear_screen();

    println!("RustOS booting...");
    println!("VGA text mode: 80x25, 16 colors");
    
    // Test serial output
    if let Some(serial_port) = &serial::SERIAL_INIT.try_lock() {
        writeln!(serial_port, "Serial port initialized successfully").unwrap();
        
        // Send a test message through both VGA and Serial ports
        println!("Testing dual output: VGA + COM1");
    } else {
        panic!("Failed to initialize serial port!");
    }

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try both VGA and Serial for the panic message
    println!("KERNEL PANIC: {}", info);
    
    if let Some(serial_port) = &serial::SERIAL_INIT.try_lock() {
        writeln!(serial_port, "KERNEL PANIC: {}", info).unwrap();
    }
    
    loop {
        core::hint::spin_loop();
    }
}
