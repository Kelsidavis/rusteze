// Global Descriptor Table implementation for RustOS

use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, SegmentSelector};
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

// Stack used when handling double faults (critical error)
#[repr(align16)]
struct DoubleFaultStack {
    data: [u8; 512],
}

static mut DOUBLE_FAULT_STACK: DoubleFaultStack = DoubleFaultStack { data: [0; 512] };

pub struct Gdt {
    gdt: GlobalDescriptorTable,
}

impl Gdt {
    pub fn new() -> Self {
        let mut gdt = GlobalDescriptorTable::new();
        
        // Kernel code segment - ring 0, readable/executable, present
        gdt.add_entry(1, x86_64::structures::gdt::Entry::kernel_code_segment());
        
        // Kernel data segment - ring 0, writable/readable, present  
        gdt.add_entry(2, x86_64::structures::gdt::Entry::kernel_data_segment());

        // User code segment - ring 3, readable/executable, present
        gdt.add_entry(3, x86_64::structures::gdt::Entry::user_code_segment());
        
        // User data segment - ring 3, writable/readable, present  
        gdt.add_entry(4, x86_64::structures::gdt::Entry::user_data_segment());

        Self { gdt }
    }

    pub fn load(&self) {
        unsafe {
            self.gdt.load();
            
            // Set up segment selectors for code and data segments in ring 0 (kernel)
            let kernel_code_selector = SegmentSelector(1 << 3);
            let kernel_data_selector = SegmentSelector(2 << 3);

            x86_64::instructions::segmentation::set_cs(kernel_code_selector);
            
            // Set up the stack pointer for user mode
            x86_64::instructions::interrupts::enable();
        }
    }

    pub fn init_gdt() {
        let mut gdt = Self::new();

        unsafe {
            // Initialize the TSS and add it to GDT
            let tss = TaskStateSegment::empty();
            
            // Set up IST (Interrupt Stack Table) for double fault handling
            let mut tss_copy = tss;
            tss_copy.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
                VirtAddr::from_ptr(&mut DOUBLE_FAULT_STACK);

            gdt.gdt.add_entry(5, x86_64::structures::gdt::Entry::tss_segment(&tss_copy));
        }

        // Load the GDT into processor's GDTR register
        unsafe {
            gdt.load();
            
            // Set up segment selectors for code and data segments in ring 0 (kernel)
            let kernel_code_selector = SegmentSelector(1 << 3);
            x86_64::instructions::segmentation::set_cs(kernel_code_selector);

            // Enable interrupts
            x86_64::instructions::interrupts::enable();
        }
    }

    pub fn add_entry(&mut self, index: usize, entry: x86_64::structures::gdt::Entry) {
        if index < 5 { 
            self.gdt.add_entry(index + 1, entry);
        } else {
            panic!("GDT entry index out of bounds");
        }
    }

    pub fn get_selector(&self, index: usize) -> SegmentSelector {
        let offset = (index as u64 * 8) + 0x2;
        unsafe { 
            core::ptr::read_volatile((offset as *const _) as *mut SegmentSelector)
        }
    }
}

// Implement the necessary traits for GDT
impl core::fmt::Debug for Gdt {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "GlobalDescriptorTable {{ entries: {} }}", 5)
    }
}
```