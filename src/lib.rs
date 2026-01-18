#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod vga;
mod serial;
mod gdt;
mod idt;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    // Initialize GDT first (required for IDT)
    unsafe {
        gdt::init_gdt();
    }

    // Initialize PIC (remap IRQs to 32-47)
    idt::init_pic();

    // Initialize and load the IDT
    idt::init_idt();

    vga::WRITER.lock().clear_screen();

    println!("RustOS booting...");
    println!("VGA text mode: 80x25, 16 colors");

    // Test serial output
    serial_println!("Serial port initialized successfully");
    serial_println!("Testing dual output: VGA + COM1");
    println!("Serial port: COM1 @ 9600 baud");
    println!("IDT and PIC initialized");

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("KERNEL PANIC: {}", info);
    serial_println!("KERNEL PANIC: {}", info);

    loop {
        core::hint::spin_loop();
    }
}
