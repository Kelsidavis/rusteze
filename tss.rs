// Task State Segment implementation for RustOS

use x86_64::structures::tss::TaskStateSegment;
use core::arch::{asm, global_asm};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

#[repr(align16)]
struct DoubleFaultStack {
    data: [u8; 512],
}

static mut DOUBLE_FAULT_STACK: DoubleFaultStack = DoubleFaultStack { data: [0; 512] };

// Initialize the TSS and set up IST (Interrupt Stack Table)
pub fn init_tss() -> TaskStateSegment {
    let mut tss = unsafe { 
        core::ptr::read_volatile(&DOUBLE_FAULT_STACK as *const _ as *mut TaskStateSegment) 
    };
    
    // Set up IST for double fault handling
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
        x86_64::VirtAddr::from_ptr(unsafe { &mut DOUBLE_FAULT_STACK });

    unsafe {
        core::ptr::write_volatile(&tss, TaskStateSegment::empty());
        
        // Load the TSS into processor's TR register
        let tss_addr = (&*core::ptr::addr_of!(tss)) as *const _;
        x86_64::instructions::tables::ltr(tss_addr as u16);
    }

    tss
}

// Global function to initialize the TSS and load it into processor's TR register
pub fn init_tss_and_load() {
    let mut tss = TaskStateSegment::empty();
    
    // Set up IST for double fault handling (critical error)
    unsafe { 
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
            x86_64::VirtAddr::from_ptr(&mut DOUBLE_FAULT_STACK);
        
        core::ptr::write_volatile(
            &tss,
            TaskStateSegment {
                // Initialize all fields to zero
                ..TaskStateSegment::empty()
            }
        );
    }

    unsafe { 
        x86_64::instructions::tables::ltr(&tss as *const _ as u16);
        
        // Set up the TSS in GDT (we'll do this when we implement GDT properly)
        let tss_addr = &tss as *const TaskStateSegment;
        core::ptr::write_volatile(
            &(x86_64::structures::gdt::Entry::empty() as *mut _),
            x86_64::structures::gdt::Entry::tss_segment(&*core::ptr::addr_of!(tss))
        );
    }
}

// This is a global assembly function to handle the TSS loading
global_asm!(
r#"
    .section .text

    // Function: tss_load
    // Loads Task State Segment into processor's TR register
    //
    // Input:
    //   rdi - pointer to TaskStateSegment structure (64-bit)
    //
    // Output:
    //   None, but modifies the task state segment selector in the TSS
    
tss_load:
    mov %rdi, %rax

    // Load TR register with address of tss
    ltr (%rax)

    ret
"#
);

// This is a global assembly function to handle double fault interrupt stack switching
global_asm!(
r#"
    .section .text

    // Function: set_double_fault_stack_pointer
    // Sets the pointer for the double fault IST entry
    
    // Input:
    //   rdi - pointer to DoubleFaultStack structure (64-bit)
    
set_double_fault_stack_pointer:
    mov %rdi, %rax
    
    // Set up interrupt stack table at index 0 with this address
    lea (%rax), %rcx

    // Store the value in TSS IST array for double fault handler
    movq %rcx, (16 * 8) + tss_data(%rip)

    ret
"#
);

// This is a global assembly function to handle interrupt stack switching
global_asm!(
r#"
    .section .text

    // Function: switch_to_interrupt_stack
    // Switches the current execution context to use an IST entry
    
    // Input:
    //   rdi - index of IST table (0-7)
    
switch_to_interrupt_stack:
    mov %rdi, %rax
    
    // Load TSS address from TR register and get pointer to interrupt stack table
    ltr (%rip)  # This is a placeholder; actual implementation requires proper addressing

    ret
"#
);
