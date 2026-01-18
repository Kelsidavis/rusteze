use x86_64::structures::idt::{InterruptDescriptorTable, ExceptionStackFrame};
use crate::gdt;
use lazy_static::lazy_static;

// Define interrupt handler function signature
pub extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("EXCEPTION: Division By Zero\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn debug_exception_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("EXCEPTION: Debug\n{:#?}", stack_frame);
}

// Define a handler for the general protection fault
pub extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: &mut ExceptionStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: General Protection Fault\nError Code: {}\n{:#?}",
        error_code, stack_frame
    );
}

// Define a handler for the page fault exception
pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut ExceptionStackFrame,
    error_code: u64,
) {
    let address = x86_64::registers::control::Cr2::read();
    
    panic!(
        "EXCEPTION: Page Fault\nError Code: {}\nFaulting Address: {:#?}\n{:#?}",
        error_code, address, stack_frame
    );
}

// Define a handler for the double fault exception (critical)
pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut ExceptionStackFrame,
    _error_code: u64,
) {
    panic!(
        "EXCEPTION: Double Fault\n{:#?}",
        stack_frame
    );
}

// Define a handler for the machine check exception (critical)
pub extern "x86-interrupt" fn machine_check_handler(
    stack_frame: &mut ExceptionStackFrame,
    _error_code: u64,
) {
    panic!(
        "EXCEPTION: Machine Check\n{:#?}",
        stack_frame
    );
}

// Define a handler for the keyboard interrupt (IRQ1)
pub extern "x86-interrupt" fn keyboard_interrupt_handler(
    stack_frame: &mut ExceptionStackFrame,
) {
    // Read from PS/2 port 0x60 to clear the interrupt
    let mut data = unsafe { x86_64::instructions::port::Port::<u8>::new(0x60).read() };
    
    println!("KEYBOARD INTERRUPT: {}", data);
}

// Define a handler for timer interrupts (IRQ0)
pub extern "x86-interrupt" fn timer_interrupt_handler(
    stack_frame: &mut ExceptionStackFrame,
) {
    // Increment tick counter or handle scheduling
    let mut ticks = unsafe { crate::TIMER_TICKS.get_mut() };
    
    if let Some(ticks_ref) = ticks.as_mut() {
        *ticks_ref += 1;
        
        // Print every second (assuming timer fires at ~100Hz)
        if (*ticks_ref % 100) == 0 {
            println!("Timer tick: {}", *ticks_ref);
            
            // Simple test - clear screen after a few seconds
            if *ticks_ref > 500 {
                vga::WRITER.lock().clear_screen();
                println!("Screen cleared at {} ticks", *ticks_ref);
                
                // Reset counter for next cycle
                *ticks_ref = 0;
            }
        }
    }

    unsafe { 
        // Send EOI (End of Interrupt) to PIC
        x86_64::instructions::port::Port::<u8>::new(0x20).write(0x20);
        
        // If using APIC, send IPI instead - but for now we'll use legacy PIC
    }

    // Acknowledge the interrupt
}

// Define a handler for serial port interrupts (IRQ4)
pub extern "x86-interrupt" fn com1_interrupt_handler(
    stack_frame: &mut ExceptionStackFrame,
) {
    let mut data = unsafe { x86_64::instructions::port::Port::<u8>::new(0x3f8).read() };
    
    // Echo back the received character
    serial::SerialPort::new(0x3F8).send_byte(data);
}

// Define a handler for system calls (int 0x80)
pub extern "x86-interrupt" fn syscall_handler(
    stack_frame: &mut ExceptionStackFrame,
) {
    // This is where we would implement actual syscalls
    println!("SYSTEM CALL HANDLER called with EAX={:#X}, EBX={:#X}", 
            stack_frame.rax, stack_frame.rbx);
    
    // For now just return (no error)
}

// Define the IDT structure and initialization function
lazy_static::lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        
        // Set up exception handlers for critical exceptions first
        idt.divide_by_zero.set_handler_fn(divide_by_zero_handler);
        idt.debug.set_handler_fn(debug_exception_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        
        // Set up machine check exception handler
        idt.machine_check.set_handler_fn(machine_check_handler);

        // Setup interrupt handlers for hardware interrupts (IRQs)
        idt[32].set_handler_fn(timer_interrupt_handler);  // IRQ0 - Timer
        idt[33].set_handler_fn(keyboard_interrupt_handler);  // IRQ1 - Keyboard
        
        // Set up serial port handler on COM1 (IRQ4) 
        idt[36].set_handler_fn(com1_interrupt_handler);
        
        // Setup system call entry point at int 0x80
        idt[0x80].set_handler_fn(syscall_handler);

        // Return the configured IDT
        idt
    };
}

// Initialize PIC (Programmable Interrupt Controller)
pub fn init_pic() {
    unsafe {
        // Send ICW1 to initialize PICs
        x86_64::instructions::port::Port::<u8>::new(0x20).write(0b00010001);  // Start initialization sequence
        
        // Set up the master interrupt vector offset (IRQ0-7)
        x86_64::instructions::port::Port::<u8>::new(0x21).write(32);
        
        // Configure cascade connection between PICs
        let mut slave_mask = 0b11111111; 
        if gdt::is_apic_enabled() {
            // If APIC is enabled, we don't need to use the cascading mode for IRQ8-15
            x86_64::instructions::port::Port::<u8>::new(0x21).write(32 | 0b00000000);
        } else {
            // Otherwise, set up slave PIC at offset 40 (IRQ8-15)
            x86_64::instructions::port::Port::<u8>::new(0x21).write(32 | 0b00000000);
            
            // Send ICW3 to master
            let mut slave_mask = 0b11111111;
        }
        
        // Set up the slave PIC (slave is connected via IRQ2)
        x86_64::instructions::port::Port::<u8>::new(0xA0).write(32 + 8);
        
        if !gdt::is_apic_enabled() {
            // Send ICW1 to slave
            let mut master_mask = 0b11111111;
            
            x86_64::instructions::port::Port::<u8>::new(0xA1).write(0b00010001); 
            
            // Set up the cascade connection between PICs (slave connected via IRQ2)
            let mut master_mask = 0b11111111;
            
            x86_64::instructions::port::Port::<u8>::new(0xA1).write(0x04); 
        }
        
        // Set up the mode (non-ICW2, non-AEOI)
        let mut master_mask = 0b11111111;
        x86_64::instructions::port::Port::<u8>::new(0x21).write(master_mask);
        
        // Set up the slave mask
        if !gdt::is_apic_enabled() {
            let mut slave_mask = 0b11111111;
            
            x86_64::instructions::port::Port::<u8>::new(0xA1).write(slave_mask);
        }
        
        // Enable the timer interrupt (IRQ0)
        if !gdt::is_apic_enabled() {
            let mut master_mask = 0b11111110;
            
            x86_64::instructions::port::Port::<u8>::new(0x21).write(master_mask);
        }
        
        // Enable the keyboard interrupt (IRQ1)
        if !gdt::is_apic_enabled() {
            let mut master_mask = 0b11111101;
            
            x86_64::instructions::port::Port::<u8>::new(0x21).write(master_mask);
        }
        
        // Enable the serial port interrupt (IRQ4)
        if !gdt::is_apic_enabled() {
            let mut master_mask = 0b11110111;
            
            x86_64::instructions::port::Port::<u8>::new(0x21).write(master_mask);
        }
    }

    // Initialize APIC if enabled
    unsafe { 
        crate::APIC_BASE.store(
            gdt::get_apic_base(),
            core::sync::atomic::Ordering::Relaxed,
        );
        
        let apic_base = crate::APIC_BASE.load(core::sync::atomic::Ordering::Relaxed);
        
        // Set up APIC for timer interrupts
        if gdt::is_apic_enabled() {
            x86_64::instructions::port::Port::<u32>::new(apic_base + 0x1B).write(0); 
            let mut apic_timer = crate::APIC_TIMER.load(core::sync::atomic::Ordering::Relaxed);
            
            // Set up APIC timer to fire every ~1ms (assuming TSC runs at ~3GHz)
            if apic_timer == 0 {
                x86_64::instructions::port::Port::<u32>::new(apic_base + 0x3E).write(0);
                
                // Set up APIC timer to fire every millisecond
                let ticks_per_ms = (gdt::get_tsc_frequency() / 1000) as u64;
                x86_64::instructions::port::Port::<u32>::new(apic_base + 0x3E).write(ticks_per_ms);
                
                // Set up APIC timer to be periodic
                let mut apic_timer = crate::APIC_TIMER.load(core::sync::atomic::Ordering::Relaxed) | (1 << 8); 
                x86_64::instructions::port::Port::<u32>::new(apic_base + 0x3E).write(ticks_per_ms);
                
                // Enable APIC timer
                let mut apic_timer = crate::APIC_TIMER.load(core::sync::atomic::Ordering::Relaxed) | (1 << 8); 
            }
        } else {
            x86_64::instructions::port::Port::<u32>::new(apic_base + 0x1B).write(0);
            
            // Set up APIC timer to fire every ~1ms
            let ticks_per_ms = (gdt::get_tsc_frequency() / 1000) as u64;
            x86_64::instructions::port::Port::<u32>::new(apic_base + 0x3E).write(ticks_per_ms);
            
            // Set up APIC timer to be periodic
            let mut apic_timer = crate::APIC_TIMER.load(core::sync::atomic::Ordering::Relaxed) | (1 << 8); 
        }
    }

    println!("PIC initialized successfully");
}

// Initialize the IDT and set it in memory
pub fn initialize() {
    // Load the interrupt descriptor table into hardware
    unsafe { IDT.load(); }
    
    // Enable interrupts after setting up handlers
    x86_64::instructions::interrupts::enable();
}
