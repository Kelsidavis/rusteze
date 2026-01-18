// Interrupt Descriptor Table for x86_64

use core::arch::asm;
use lazy_static::lazy_static;
use spin::{Mutex, Once};
use x86_64::{
    structures::idt::{InterruptDescriptorTable, ExceptionHandlerFn},
    VirtAddr,
};

/// The IDT entry structure
#[derive(Debug)]
pub struct Idt {
    idt: InterruptDescriptorTable,
}

impl Idt {
    /// Create a new empty IDT
    pub const fn new() -> Self {
        Self {
            idt: InterruptDescriptorTable::new(),
        }
    }

    /// Initialize the IDT with exception handlers and interrupt handlers
    ///
    /// # Safety
    /// This function must only be called once during kernel initialization.
    /// It modifies global state and executes privileged CPU instructions.
    pub unsafe fn initialize(&mut self) {
        // Set up all exceptions (0-31)
        self.idt.divide_by_zero.set_handler_fn(divide_by_zero_handler);
        self.idt.debug.set_handler_fn(debug_handler);
        self.idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        self.idt.breakpoint.set_handler_fn(breakpoint_handler);
        self.idt.overflow.set_handler_fn(overflow_handler);
        self.idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        self.idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        self.idt.device_not_available.set_handler_fn(device_not_available_handler);
        self.idt.double_fault.set_handler_fn(double_fault_handler);
        self.idt.coprocessor_segment_overrun.set_handler_fn(coprocessor_segment_overrun_handler);
        self.idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        self.idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        self.idt.stack_segment_error.set_handler_fn(stack_segment_error_handler);
        self.idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        self.idt.page_fault.set_handler_fn(page_fault_handler);
        // Reserved exceptions (32-47) - we'll handle them as interrupts
        for i in 32..=47 {
            let handler = interrupt_handler;
            if i == 8 { 
                continue; // Double fault is handled separately above
            }
            self.idt[i].set_handler_fn(handler);
        }

        // Set up timer (IRQ0) and keyboard interrupts
        self.idt[32].set_handler_fn(timer_interrupt_handler);   // IRQ0 - PIT
        self.idt[33].set_handler_fn(keyboard_interrupt_handler);  // IRQ1 - Keyboard

        // Load the IDT into CPU
        asm!(
            "lidt [{}]",
            in(reg) &self.idt,
            options(nostack, preserves_flags)
        );
    }

    /// Set up a handler for an interrupt vector (used by PIC setup later)
    pub fn set_handler(&mut self, irq: u8, handler_fn: ExceptionHandlerFn) {
        let index = 32 + irq as usize;
        if index < 47 { // Only handle IRQs that are not reserved
            self.idt[index].set_handler_fn(handler_fn);
        }
    }

    /// Get a reference to the IDT for use in other modules
    pub fn get_idt(&self) -> &InterruptDescriptorTable {
        &self.idt
    }
}

// Exception handlers - these will be called when an exception occurs

extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: x86_64::structures::idt::ExceptionStackFrame) {
    println!("EXCEPTION: Divide by zero");
    panic!("Divide by zero error!");
}

extern "x86-interrupt" fn debug_handler(stack_frame: x86_64::structures::idt::ExceptionStackFrame) {
    println!("DEBUG EXCEPTION:");
    print_stack_trace(&stack_frame);
    // We don't want to crash on a debug exception
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
) {
    println!("EXCEPTION: Non-maskable interrupt");
    panic!("Non-maskable interrupt!");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: x86_64::structures::idt::ExceptionStackFrame) {
    // This is called when a debug break point instruction (int 3) occurs
    println!("DEBUG BREAKPOINT:");
    print_stack_trace(&stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: x86_64::structures::idt::ExceptionStackFrame) {
    println!("EXCEPTION: Overflow");
    panic!("Overflow error!");
}

extern "x86-interrupt" fn bound_range_exceeded_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
) {
    println!("EXCEPTION: Bound range exceeded");
    panic!("Bound range exceeded!");
}

extern "x86-interrupt" fn invalid_opcode_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
) {
    println!("EXCEPTION: Invalid opcode");
    panic!("Invalid opcode error!");
}

extern "x86-interrupt" fn device_not_available_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
) {
    // This is called when a FPU instruction is executed but the coprocessor isn't available
    println!("EXCEPTION: Device not available");
    panic!("Device not available!");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
    _error_code: u64,
) {
    // This is called when a second exception occurs during the handling of another
    println!("EXCEPTION: Double fault");
    panic!("Double fault!");
}

extern "x86-interrupt" fn coprocessor_segment_overrun_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
) {
    // This is called when the FPU segment overruns
    println!("EXCEPTION: Coprocessor segment overrun");
    panic!("Coprocessor segment overrun!");
}

extern "x86-interrupt" fn invalid_tss_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
    _error_code: u64,
) {
    println!("EXCEPTION: Invalid TSS");
    panic!("Invalid TSS error!");
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
    _error_code: u64,
) {
    // This is called when a segment selector points to an invalid or non-present descriptor
    println!("EXCEPTION: Segment not present");
    panic!("Segment not present!");
}

extern "x86-interrupt" fn stack_segment_error_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
    _error_code: u64,
) {
    // This is called when a stack segment error occurs (e.g., invalid SS selector)
    println!("EXCEPTION: Stack segment error");
    panic!("Stack segment error!");
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
    _error_code: u64,
) {
    // This is called when a protection violation occurs (e.g., invalid memory access)
    println!("EXCEPTION: General protection fault");
    panic!("General protection fault!");
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: x86_64::structures::idt::ExceptionStackFrame,
    error_code: u64,
) {
    // This is called when a memory access violates the paging rules
    let addr = unsafe { x86_64::registers::control::Cr2::read() };
    
    println!("EXCEPTION: Page fault at address {:x}", addr);
    if (error_code & 1) != 0 {
        // Present bit is not set - page doesn't exist
        println!("Page does not exist");
    } else if error_code & 4 != 0 {
        // Write access violation
        println!("Write protection fault");
    }
    
    panic!("Page fault!");
}

// Interrupt handlers for hardware interrupts

extern "x86-interrupt" fn timer_interrupt_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    // This is called every 10ms (when the PIT fires)
    println!("TIMER INTERRUPT");
    
    unsafe { 
        let mut pic = PIC.lock();
        pic.eoi(0); // End of interrupt for IRQ0
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    println!("KEYBOARD INTERRUPT");
    
    unsafe { 
        let mut pic = PIC.lock();
        pic.eoi(1); // End of interrupt for IRQ1
    }
}

// Helper function to print stack trace (for debugging)
fn print_stack_trace(stack_frame: &x86_64::structures::idt::ExceptionStackFrame) {
    println!("RIP: {:p}", stack_frame.rip);
    println!("CS: 0x{:x}", stack_frame.cs);
    println!("RFLAGS: 0x{:x}", stack_frame.rflags);
    println!("RAX: 0x{:x}", stack_frame.rax);
    println!("RBX: 0x{:x}", stack_frame.rbx);
    println!("RCX: 0x{:x}", stack_frame.rcx);
    println!("RDX: 0x{:x}", stack_frame.rdx);
    println!("RSI: 0x{:x}", stack_frame.rsi);
    println!("RDI: 0x{:x}", stack_frame.rdi);
    println!("RBP: 0x{:x}", stack_frame.rbp);
    println!("RSP: 0x{:x}", stack_frame.rsp);
}

// Static IDT instance
static mut IDT_INSTANCE: Once<Idt> = Once::new();

/// Get a reference to the global IDT (safe because it's initialized once)
pub fn idt() -> &'static Idt {
    unsafe { 
        // This is safe since we're using Once and only initializing once during boot
        &*IDT_INSTANCE.call_once(|| Idt::new())
    }
}

// Static PIC instance for managing hardware interrupts
lazy_static! {
    pub static ref PIC: Mutex<Pic> = Mutex::new(Pic::new());
}

/// Programmable Interrupt Controller (PIC) - manages external hardware interrupts
pub struct Pic {
    command_port: u16,
    data_port: u16,
}

impl Pic {
    /// Create a new instance of the PIC with default ports
    pub const fn new() -> Self {
        Pic { 
            command_port: 0x20, 
            data_port: 0xA0 
        }
    }

    // Send an EOI (End Of Interrupt) signal to the PICs
    /// This tells the PIC that we've handled this interrupt and it can accept new ones.
    pub fn eoi(&mut self, irq_number: u8) {
        unsafe { 
            if irq_number < 8 {
                core::ptr::write_volatile(self.command_port as *mut u8, 0x20);
            } else {
                // Send EOI to both PICs
                core::ptr::write_volatile(0xA0 as *mut u8, 0x20); 
                core::ptr::write_volatile(0x20 as *mut u8, 0x20);
            }
        }
    }

    /// Initialize the PIC with proper settings
    pub fn initialize(&self) {
        // Save current masks (we'll restore them later)
        let mask1 = unsafe { core::ptr::read_volatile(self.data_port as *const u8) };
        let mask2 = unsafe { core::ptr::read_volatile((0xA1) as *const u8) };

        // ICW1 - Initialize command word
        unsafe {
            core::ptr::write_volatile(0x20 as *mut u8, 0b1001_0001); // Start initialization sequence

            // Set up the vector offsets (IRQs start at 32)
            core::ptr::write_volatile(0x21 as *mut u8, 32);
            
            // ICW3 - Cascade configuration
            core::ptr::write_volatile(0x21 as *mut u8, 4); // IRQ2 is connected to slave PIC

            // ICW4 - Set up for x86 mode (not in special fully nested mode)
            core::ptr::write_volatile(0x21 as *mut u8, 0b0000_0100);

            // Initialize the second PIC
            core::ptr::write_volatile(0xA0 as *mut u8, 0b1001_0001); 
            core::ptr::write_volatile(0xA1 as *mut u8, 32 + 8);
            core::ptr::write_volatile(0xA1 as *mut u8, 4); // Slave PIC connected to IRQ2
            core::ptr::write_volatile(0xA1 as *const u8, 0b0000_0100);

            // Restore masks (disable all interrupts)
            core::ptr::write_volatile(self.data_port as *mut u8, mask1);
            core::ptr::write_volatile((self.data_port + 1) as *mut u8, mask2); 
        }
    }

    /// Disable the PICs
    pub fn disable(&self) {
        unsafe { 
            // Write to both data ports with all bits set (disable interrupts)
            core::ptr::write_volatile(self.data_port as *mut u8, 0xFF);
            core::ptr::write_volatile((self.data_port + 1) as *mut u8, 0xFF); 
        }
    }

    /// Set the interrupt mask for a specific IRQ
    pub fn set_mask(&self, irq: u8) {
        let port = if irq < 8 { self.data_port } else { (self.data_port + 1) };
        
        unsafe {
            // Read current mask and clear bit corresponding to this IRQ
            let mut new_mask = core::ptr::read_volatile(port as *const u8);
            
            // Set the interrupt enable flag for this IRQ by clearing its bit in the mask
            if irq < 8 { 
                new_mask |= (1 << irq); 
            } else {
                new_mask |= (1 << (irq - 8));
            }
            
            core::ptr::write_volatile(port as *mut u8, new_mask);
        }
    }

    /// Clear the interrupt mask for a specific IRQ
    pub fn clear_mask(&self, irq: u8) {
        let port = if irq < 8 { self.data_port } else { (self.data_port + 1) };
        
        unsafe {
            // Read current mask and set bit corresponding to this IRQ 
            let mut new_mask = core::ptr::read_volatile(port as *const u8);
            
            // Clear the interrupt enable flag for this IRQ by setting its bit in the mask
            if irq < 8 { 
                new_mask &= !(1 << irq); 
            } else {
                new_mask &= !(1 << (irq - 8));
            }
            
            core::ptr::write_volatile(port as *mut u8, new_mask);
        }
    }

    /// Get the current interrupt mask
    pub fn get_mask(&self) -> (u8, u8) {
        unsafe { 
            let mask1 = core::ptr::read_volatile(self.data_port as *const u8);
            let mask2 = core::ptr::read_volatile((0xA1) as *const u8);  
            
            (mask1, mask2)
        }
    }

    /// Set the interrupt masks for all IRQs
    pub fn set_masks(&self, irq_mask: &[u8]) {
        unsafe { 
            // Write to both data ports with new masks
            core::ptr::write_volatile(self.data_port as *mut u8, irq_mask[0]);
            core::ptr::write_volatile((self.data_port + 1) as *mut u8, irq_mask[1]);  
        }
    }

    /// Get the current interrupt mask for a specific IRQ
    pub fn is_enabled(&self, irq: u8) -> bool {
        let port = if irq < 8 { self.data_port } else { (self.data_port + 1) };
        
        unsafe {
            // Read from appropriate data port and check bit corresponding to this IRQ
            let mask = core::ptr::read_volatile(port as *const u8);
            
            if irq < 8 {
                return ((mask >> irq) & 0x01) == 0;
            } else {
                return ((mask >> (irq - 8)) & 0x01) == 0;
            }
        }
    }

    /// Enable a specific IRQ
    pub fn enable_irq(&self, irq: u8) {
        self.clear_mask(irq);
    }

    /// Disable a specific IRQ  
    pub fn disable_irq(&self, irq: u8) {
        self.set_mask(irq);
    }
}

// Initialize the PIC when we start up (called from kernel_main)
pub unsafe fn init_pic() {
    let mut pic = PIC.lock();
    
    // Set up interrupt vectors for all 16 IRQs
    idt().set_handler(0, timer_interrupt_handler);   // Timer - IRQ0  
    idt().set_handler(1, keyboard_interrupt_handler); // Keyboard - IRQ1
    
    // Initialize the PIC with proper settings (this will set vector offsets)
    pic.initialize();
    
    // Enable only necessary interrupts
    pic.enable_irq(0);  // Timer interrupt enabled
    pic.enable_irq(1);  // Keyboard interrupt enabled
}
