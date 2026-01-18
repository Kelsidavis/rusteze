use x86_64::structures::idt::{InterruptDescriptorTable, ExceptionStackFrame};
use crate::gdt;
use lazy_static::lazy_static;

// Define interrupt handler function signature
pub extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("EXCEPTION: Divide by zero\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn debug_exception_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("DEBUG EXCEPTION: {}", stack_frame);
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
    // Read scan code from PS/2 port 0x60
    let scancode = unsafe { x86_64::instructions::port::Port::<u8>::new(0x60).read() };
    
    println!("KEYBOARD INTERRUPT: Scan Code {}", scancode);
}

// Define a handler for the timer interrupt (IRQ0)
pub extern "x86-interrupt" fn timer_interrupt_handler(
    stack_frame: &mut ExceptionStackFrame,
) {
    // Increment tick counter
    crate::timer::TICKS.lock().add(1);

    println!("TIMER INTERRUPT - Tick count increased");
    
    unsafe { 
        // Send EOI (End of Interrupt) to PIC
        x86_64::instructions::port::Port::<u8>::new(0x20).write(0x20);
        
        // If using APIC, send IPI instead - but for now we'll use legacy PIC
    }
}

// Define a handler for the system call interrupt (int 0x80)
pub extern "x86-interrupt" fn syscall_handler(
    stack_frame: &mut ExceptionStackFrame,
) {
    // This is where syscalls will be handled in future phases
    
    println!("SYSTEM CALL - Not implemented yet");
}

// Define a handler for the hardware interrupt (IRQ2-15)
pub extern "x86-interrupt" fn irq_handler(
    stack_frame: &mut ExceptionStackFrame,
) {
    // Read IRQ number from PIC
    let irq = unsafe { x86_64::instructions::port::Port::<u8>::new(0x20).read() };
    
    println!("IRQ HANDLER - Interrupt {} received", irq);
}

// Define the IDT structure and initialization function
pub struct Idt {
    idt: InterruptDescriptorTable,
}

impl Idt {
    pub fn new() -> Self {
        let mut idt = InterruptDescriptorTable::new();
        
        // Set up exception handlers (0-31)
        idt.divide_by_zero.set_handler_fn(divide_by_zero_handler);
        idt.debug_exception.set_handler_fn(debug_exception_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);

        // Set up system call handler (int 0x80)
        idt[47].set_handler_fn(syscall_handler); 

        // Set up hardware interrupt handlers for IRQs
        idt[32].set_handler_fn(timer_interrupt_handler);
        idt[33].set_handler_fn(keyboard_interrupt_handler);

        // For other interrupts, use a generic handler (will be refined later)
        let mut i = 0;
        while i < 16 {
            if !(i == 8 || i == 9) { 
                idt[i + 32].set_handler_fn(irq_handler);
            }
            i += 1;
        }

        Idt { idt }
    }

    pub fn initialize(&self) {
        self.idt.load();
        
        // Set up the IDT pointer for use in other modules
        unsafe {
            crate::idt_ptr = Some(self as *const Self);
        }
    }
}

// Initialize PIC (Programmable Interrupt Controller)
pub fn init_pic() {
    // Send ICW1 to initialize PICs
    unsafe { 
        x86_64::instructions::port::Port::<u8>::new(0x20).write(0b00010001);  // ICW1: Start initialization, edge-triggered mode
        x86_64::instructions::port::Port::<u8>::new(0xA0).write(0b00010001);
        
        // Send ICW2 - set interrupt vector offsets (IRQs 32-47 for master, 56-79 for slave)
        x86_64::instructions::port::Port::<u8>::new(0x21).write(0b00000000); // Master PIC: IRQ vectors start at 0x20
        x86_64::instructions::port::Port::<u8>::new(0xA1).write(0b00000000); // Slave PIC: IRQ vectors start at 0x30
        
        // Send ICW3 - connect slave to master (IRQ2)
        x86_64::instructions::port::Port::<u8>::new(0x21).write(0b00000010); 
        x86_64::instructions::port::Port::<u8>::new(0xA1).write(0b00000000);
        
        // Send ICW4 - set mode (8259A compatibility)
        x86_64::instructions::port::Port::<u8>::new(0x21).write(0b00000100); 
        x86_64::instructions::port::Port::<u8>::new(0xA1).write(0b00000100);
        
        // Mask all interrupts initially (except for timer and keyboard)
        unsafe {
            let mut master_mask = 0xFF;
            let mut slave_mask = 0xFF;

            // Unmask only the necessary IRQs: Timer (IRQ0), Keyboard (IRQ1) 
            master_mask &= !(1 << 0);   // Enable timer
            master_mask &= !(1 << 1);   // Enable keyboard
            
            x86_64::instructions::port::Port::<u8>::new(0x21).write(master_mask);
            
            slave_mask &= !(1 << 0);    // Enable cascade (IRQ2)
            x86_64::instructions::port::Port::<u8>::new(0xA1).write(slave_mask);
        }
        
        println!("PIC initialized successfully");
    }

    pub fn enable_irq(&self, irq: u8) {
        unsafe { 
            let port = if irq < 8 { 0x21 } else { 0xA1 };
            
            // Read current mask
            let mut mask = x86_64::instructions::port::Port::<u8>::new(port).read();
            
            // Clear the bit for this IRQ (enable it)
            mask &= !(1 << (irq % 8));
            
            // Write back to port
            x86_64::instructions::port::Port::<u8>::new(port).write(mask);
        }
    }

    pub fn disable_irq(&self, irq: u8) {
        unsafe { 
            let port = if irq < 8 { 0x21 } else { 0xA1 };
            
            // Read current mask
            let mut mask = x86_64::instructions::port::Port::<u8>::new(port).read();
            
            // Set the bit for this IRQ (disable it)
            mask |= (1 << (irq % 8));
            
            // Write back to port
            x86_64::instructions::port::Port::<u8>::new(port).write(mask);
        }
    }

}

// Export a global pointer so other modules can access the IDT
pub static mut idt_ptr: Option<&'static Idt> = None;

lazy_static! {
    pub static ref IDT_INSTANCE: Idt = Idt::new();
}
