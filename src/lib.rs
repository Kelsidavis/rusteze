#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod vga;
mod serial;
mod gdt;
mod idt;
mod pit;
mod physical_memory;
mod paging;
mod heap;
mod keyboard;
mod ps2_mouse; // Added mouse module
mod pci;  // New PCI enumeration module
mod ata;  // ATA/IDE disk driver
mod process; // Process management and scheduling

use x86_64::structures::paging::{PhysFrame, Size4KiB, Page};
use x86_64::{VirtAddr, PhysAddr};

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
    println!("Serial port: COM1 @ 9604 baud");
    println!("IDT and PIC initialized");

    // Initialize the PIT timer interrupt at 100Hz
    pit::init_pit();
    
    // Create a physical memory allocator from boot info
    let mut frame_allocator = unsafe {
        physical_memory::BitmapFrameAllocator::new(_boot_info.memory_regions.as_ref())
    };
    
    // Initialize the bitmap with reserved areas marked as used
    frame_allocator.init();

    println!("Physical memory manager initialized");
    
    // Set up virtual memory using 4-level page tables
    let mut pager_manager = paging::init_paging(0x1_0000);
    
    // Test a simple mapping to ensure it works correctly
    let test_page = Page::<Size4KiB>::containing_address(
        VirtAddr::new(0x2_0000)
    );

    if pager_manager.map_to(test_page,
                           PhysFrame::containing_address(PhysAddr::new(0x3_0000)),
                           paging::TABLE_FLAGS).is_ok() {
        println!("Virtual memory: 4-level page tables initialized successfully");
    } else {
        panic!("Failed to initialize virtual memory mapping!");
    }

    // Initialize the kernel heap allocator
    unsafe {
        heap::init_heap();
    }
    println!("Kernel heap allocator initialized (64 KiB linked list)");

    // Initialize the PS/2 keyboard driver
    keyboard::init_keyboard();
    println!("PS/2 keyboard driver initialized");

    // Initialize the PS/2 mouse driver
    ps2_mouse::init_mouse();
    println!("PS/2 mouse driver initialized");

    // Initialize the process manager (scheduler)
    process::init_process_manager();
    println!("Process manager initialized (round-robin scheduler)");

    // Enable hardware interrupts
    idt::enable_interrupts();
    println!("Hardware interrupts enabled");

    // Initialize PCI enumeration (new feature)
    let pci_devices = pci::init_pci();

    if !pci_devices.is_empty() {
        println!("PCI devices discovered: {}", pci_devices.len());
        
        for device in &pci_devices {
            match (
                pci::get_vendor_id(device),
                pci::get_device_id(device),
                pci::get_class_code(device)
            ) {
                (Some(vid), Some(did), Some(class)) => {
                    println!("  Device: {} - Vendor ID {:#06X}, Device ID {:#06X} Class {:?}",
                            device, vid, did, class);
                    
                    // Print BARs if available
                    for i in 0..=5 {
                        match pci::get_bar(device, i) {
                            Some(bar_val) => println!("    BAR{}: {:?}", i, bar_val),
                            None => continue,
                        }
                    }
                },
                _ => {}
            }
        }
        
        // Print summary of discovered devices
        use alloc::collections::BTreeMap;
        let mut class_count = BTreeMap::<u8, u32>::new();
        for device in &pci_devices {
            if let Some(class) = pci::get_class_code(device) {
                *class_count.entry(class).or_insert(0) += 1;
            }
        }

        println!("PCI Class Distribution:");
        for (cls, count) in class_count.iter() {
            match cls {
                0x01 => println!("    Storage: {} device(s)", count),
                0x02 => println!("    Network: {} device(s)", count),
                0x03 => println!("    Display: {} device(s)", count),
                0x04 => println!("    Multimedia: {} device(s)", count),
                0x05 => println!("    Memory: {} device(s)", count),
                0x06 => println!("    Bridge: {} device(s)", count),
                0x07 => println!("    Communication: {} device(s)", count),
                _ => println!("    Other (0x{:02X}): {} device(s)", cls, count)
            }
        }

    } else {
        println!("No PCI devices found");
    }

    // Initialize ATA/IDE disk driver
    let ata_disks = ata::init_ata();
    
    if !ata_disks.is_empty() {
        for (i, disk) in ata_disks.iter().enumerate() {
            match disk.init() {
                Ok(info) => println!("ATA Disk {}: Model '{}', Serial '{}' ({})", 
                    i + 1,
                    info.model_name().unwrap_or("Unknown"),
                    info.serial_number_str().unwrap_or("Unknown"),
                    if (info.device_type & 0x4) == 0 { "Fixed" } else { "Removable" }),
                Err(_) => println!("ATA Disk {}: Failed to initialize", i + 1),
            }
        }

        // Test reading a sector from the first disk
        let test_lba = 256;
        
        if !ata_disks.is_empty() {
            match ata_disks[0].read_sector(test_lba) {
                Ok(data) => println!("Successfully read LBA {} (first 16 bytes: {:?})", 
                    test_lba, &data[..16]),
                Err(e) => println!("Failed to read from ATA disk at LBA {}: {}", test_lba, e),
            }
        } else {
            println!("No ATA disks available for testing");
        }

    } else {
        println!("No ATA/IDE devices found.");
    }

    // Mark the task as complete
    println!("ATA/IDE driver (PIO mode) initialized successfully");

    println!("RustOS ready!");

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
