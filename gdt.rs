// GDT implementation for RustOS

use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

// Initialize the GDT and TSS
pub fn init_gdt() {
    let mut gdt = GlobalDescriptorTable::new();
    
    // Kernel code segment - ring 0, readable/executable, present
    gdt.add_entry(Descriptor::kernel_code_segment());
    
    // Kernel data segment - ring 0, writable/readable, present  
    gdt.add_entry(Descriptor::kernel_data_segment());
    
    // User code segment - ring 3, readable/executable, present
    gdt.add_entry(Descriptor::user_code_segment());
    
    // User data segment - ring 3, writable/readable, present
    gdt.add_entry(Descriptor::user_data_segment());

    unsafe {
        // Initialize the TSS and add it to GDT
        let mut tss = TaskStateSegment::new();
        
        // Set up IST (Interrupt Stack Table) for double fault handling
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
            VirtAddr::from_ptr(unsafe { &mut DOUBLE_FAULT_STACK });

        gdt.add_entry(Descriptor::tss_segment(&tss));
    }

    unsafe {
        // Load the GDT into processor's GDTR register
        gdt.load();
        
        // Set up segment selectors for code and data segments in ring 0 (kernel)
        let kernel_code_selector = SegmentSelector(1 << 3);
        let kernel_data_selector = SegmentSelector(2 << 3);

        x86_64::instructions::segmentation::set_cs(kernel_code_selector);
        
        // Set up the stack pointer for user mode
        x86_64::instructions::interrupts::enable();
    }
}

// Stack used when handling double faults (critical error)
#[repr(align16)]
pub struct DoubleFaultStack {
    data: [u8; 512],
}

static mut DOUBLE_FAULT_STACK: DoubleFaultStack = DoubleFaultStack { data: [0; 512] };

