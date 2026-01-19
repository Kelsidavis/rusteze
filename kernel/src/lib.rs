#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use x86_64::{
    structures::paging::{PhysFrame, Size4KiB},
    VirtAddr,
};

mod vga;
mod serial;
mod gdt;
mod idt;
mod pit;
mod physical_memory;
mod paging;
mod heap;
mod keyboard;
mod ps2_mouse;
mod pci;
mod ata;
mod process;
mod syscall;
mod vfs;
mod tmpfs;
mod devfs;
mod procfs;
mod initramfs;

use x86_64::structures::paging::{PhysFrame, Size4KiB};

entry_point!(kernel_main);

// ELF header structure
#[repr(C)]
pub struct ElfHeader {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16
}

// ELF program header structure
#[repr(C)]
pub struct ElfProgramHeader {
    pub p_type: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_flags: u32,
    pub p_align: u64
}

// ELF loading errors
#[derive(Debug)]
pub enum ElfError {
    InvalidMagic,
    UnsupportedArchitecture,
    BadProgramHeader,
    MemoryAllocationFailed,
    LoadFailure,
}

impl core::fmt::Display for ElfError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "ELF file has invalid magic number"),
            Self::UnsupportedArchitecture => write!(f, "ELF architecture not supported (only x86_64 for now)"),
            Self::BadProgramHeader => write!(f, "Malformed program header in ELF binary"),
            Self::MemoryAllocationFailed => write!(f, "Could not allocate memory for loading process segments"),
            Self::LoadFailure => write!(f, "Loading of the executable failed at runtime"),
        }
    }
}

// Load a static (non-dynamic) ELF file into virtual address space
pub fn load_elf_binary(
    elf_data: &[u8],
    process_manager: &mut crate::process::ProcessManager,
) -> Result<PhysFrame, ElfError> {
    // Validate ELF header magic bytes
    if !elf_data.starts_with(&[0x7F, b'E', b'L', b'F']) {
        return Err(ElfError::InvalidMagic);
    }

    let elf_header = unsafe { &*(elf_data.as_ptr() as *const ElfHeader) };

    // Check for 64-bit ELF (we only support x86_64)
    if elf_header.e_ident[4] != 2 {
        return Err(ElfError::UnsupportedArchitecture);
    }

    let entry_point = elf_header.e_entry;

    // Extract program headers
    let ph_offset = elf_header.e_phoff as usize;
    let mut prog_headers: Vec<ElfProgramHeader> = vec![];
    
    for i in 0..elf_header.e_phnum {
        unsafe {
            let hdr_ptr =
                (elf_data.as_ptr() as *const u8).add(ph_offset + i as usize * elf_header.e_phentsize as usize)
                    .cast::<ElfProgramHeader>();
            
            if !hdr_ptr.is_null() && (*hdr_ptr).p_type == 0x1 { // PT_LOAD segment type
                prog_headers.push(*hdr_ptr);
            }
        }
    }

    let mut total_memory_size = 0;
    
    for hdr in &prog_headers {
        total_memory_size += (hdr.p_memsz as usize + 4095) / 4096 * 4096; // Round up to page size
    }

    if total_memory_size == 0 || prog_headers.is_empty() {
        return Err(ElfError::BadProgramHeader);
    }
    
    let phys_frame = crate::physical_memory::allocate_frames(total_memory_size / 4096)
        .map_err(|_| ElfError::MemoryAllocationFailed)?;

    // Map each segment into virtual memory
    for hdr in &prog_headers {
        
        if (hdr.p_flags & 1) == 0 { continue; } // Skip non-executable segments
        
        let virt_addr = hdr.p_vaddr as usize;
        let file_offset = hdr.p_offset as usize;

        unsafe {
            // Copy the segment data from ELF binary into allocated physical memory
            for i in (virt_addr)..(virt_addr + hdr.p_filesz) {
                if !phys_frame.is_valid() || phys_frame.0 == 0 { continue; }

                let frame_ptr = core::ptr::addr_of!(phys_frame).cast::<u8>();
                
                *frame_ptr.add(i - virt_addr + file_offset) =
                    elf_data[file_offset + i - virt_addr];
            }
        }
    }

    // Create the init process (PID 1)
    let mut pcb = crate::process::ProcessControlBlock::new(
        1,
        phys_frame.clone(),
        entry_point as u64,
        true
    );

    const USER_STACK_SIZE: usize = 4096 * 8; // 32KB for user stack
    
    let user_stack_frames =
        crate::physical_memory::allocate_frames(USER_STACK_SIZE / 4096)
            .map_err(|_| ElfError::MemoryAllocationFailed)?;

    pcb.set_user_stack(user_stack_frames);

    process_manager.scheduler.add_process(pcb);
    
    Ok(phys_frame)
}

// Initialize the first userspace process (PID = 1) using a static binary
pub fn init_system() -> Result<(), ElfError> {
    // This would normally load an embedded ELF file like:
    //
    // const EMBEDDED_ELF: &[u8] = include_bytes!("../assets/initramfs.cpio");
    
    let valid_elf = &include_bytes!("../assets/test.elf")[..];
    
    if !valid_elf.starts_with(&[0x7F, b'E', b'L', b'F']) {
        return Err(ElfError::InvalidMagic);
    }

    // Initialize process manager
    let mut process_manager = crate::process::ProcessManager::new();

    load_elf_binary(valid_elf, &mut process_manager)?;

    Ok(())
}

// Kernel entry point - called after bootloader setup is complete.
fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    // Initialize GDT first (required for IDT)
    unsafe {
        gdt::init_gdt();
    }

    idt::init_pic();

    idt::init_idt();

    vga::WRITER.lock().clear_screen();

    println!("RustOS booting...");
    
    serial_println!("Serial port initialized successfully");
    serial_println!("Dual output: VGA + COM1");

    // Initialize the PIT timer interrupt at 100Hz
    pit::init_pit();
    
    let mut frame_allocator = unsafe {
        physical_memory::BitmapFrameAllocator::new(_boot_info.memory_regions.as_ref())
    };
    
    frame_allocator.init();

    println!("Physical memory manager initialized");
    
    // Set up virtual memory using 4-level page tables
    let pager_manager = paging::init_paging(0x1_0000);
    
    if pager_manager.map_to(
        x86_64::structures::paging::Page::<Size4KiB>::containing_address(x86_64::VirtAddr::new(0x2_0000)),
        PhysFrame::containing_address(x86_64::PhysAddr::new(0x3_0000)),
        paging::TABLE_FLAGS
    ).is_ok() {
        println!("Virtual memory: 4-level page tables initialized successfully");
    } else {
        panic!("Failed to initialize virtual memory mapping!");
    }

    // Initialize the kernel heap allocator
    unsafe {
        heap::init_heap();
    }
    
    keyboard::init_keyboard();

    ps2_mouse::init_mouse();

    process::init_process_manager();

    idt::enable_interrupts();

    println!("Hardware interrupts enabled");
    println!("System call interface ready (int 0x80)");

    // Initialize PCI enumeration
    let pci_devices = pci::init_pci();
    
    if !pci_devices.is_empty() {
        for device in &pci_devices {
            match (
                pci::get_vendor_id(device),
                pci::get_device_id(device),
                pci::get_class_code(device)
            ) {
                (Some(vid), Some(did), Some(class)) => {
                    println!("  Device: {} - Vendor ID {:#06X}, Device ID {:#06X} Class {:?}",
                            device, vid, did, class);
                    
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

    // Initialize VFS and tmpfs
    let tmpfs = tmpfs::TmpFs::new();
    
    if let Ok(_) = init_system() { 
        println!("Init process (PID 1) successfully loaded");
        
        loop {
            core::hint::spin_loop();
        }
    } else {
        panic!("Failed to initialize system with ELF binary!");
    }

    // Mark the task as complete
    println!("RustOS ready!");

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC: {}", info);
    
    vga::WRITER.lock().clear_screen();

    // Print the message to VGA
    for c in "RustOS KERNEL PANIC! ".bytes() {
        unsafe { 
            let writer = core::ptr::addr_of!(vga::WRITER).read();
            if !writer.is_null() && (*writer) != 0x123456789ABCDEF0u64 as *mut vga::Writer
                || (c == b'\n') {
                    let mut writer = &*writer;
                    match c { 
                        b'\n' => writer.new_line(),
                        _ => if writer.column_position < 80 {
                            // Write character to VGA buffer at current position
                            (*writer).write_byte(c);
                            writer.column_position += 1;  
                        } else {
                            writer.new_line();
                        }
                    };
                }
        }

    }

    loop {
        core::hint::spin_loop();
    }
}
