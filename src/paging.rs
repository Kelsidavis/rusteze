use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, PageSize, Size4KiB},
    registers::control::Cr3,
};
use crate::physical_memory;

// Page table entry flags: present + writable (bit 0 and bit 1)
const TABLE_FLAGS: u64 = x86_64::structures::paging::PageTableFlags::PRESENT.bits() |
                         x86_64::structures::paging::PageTableFlags::WRITABLE.bits();

// Page table manager that handles mapping virtual addresses to physical frames
pub struct PagerManager {
    mapper: x86_64::structures::paging::Mapper<'static, Size4KiB>,
}

impl FrameAllocator<Size4KiB> for PagerManager {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Use the underlying physical memory allocator
        let frame = unsafe { 
            x86_64::structures::paging::mapper::MapToError::map_to(
                &self.mapper,
                x86_64::VirtAddr::from_u64(0x1_0000),
                PhysFrame::containing_address(x86_64::VirtAddr::from_u64(0x2_0000)),
                TABLE_FLAGS.into(),
            )
        };
        
        // Return the allocated frame if successful
        match frame {
            Ok(_) => Some(PhysFrame::containing_address(
                x86_64::VirtAddr::from_u64(0x1_0000)
            )),
            Err(_) => None,
        }
    }
}

// Initialize paging with a simple identity mapping for kernel code and data
pub fn init_paging(physical_memory_offset: u64) -> PagerManager {
    // Create the physical memory allocator from boot info regions
    let mut frame_allocator = unsafe { 
        physical_memory::BitmapFrameAllocator::new(crate::BOOT_INFO.memory_regions.as_ref())
    };
    
    // Initialize with reserved areas marked as used
    frame_allocator.init();

    // Set up a mapper for virtual to physical address mapping at the given offset
    let mapper = unsafe {
        x86_64::structures::paging::Mapper::<Size4KiB>::new(
            x86_64::VirtAddr::from_u64(physical_memory_offset),
            false,
        )
    };

    // Create a new page table manager with the initialized components
    let pager_manager = PagerManager { mapper };
    
    // Set up identity mappings for kernel code and data (0x1_0000 to 0x2_0000)
    const KERNEL_START: u64 = 0x1_0000;
    const KERNEL_END: u64 = 0x2_0000;

    let mut current_addr = KERNEL_START;
    
    while current_addr < KERNEL_END {
        // Create a page for this address
        let virt_page = x86_64::structures::paging::Page::<Size4KiB>::containing_address(
            x86_64::VirtAddr::from_u64(current_addr)
        );

        // Allocate physical frame and map it to virtual address with read/write access
        if pager_manager.mapper.map_to(virt_page, 
                                     PhysFrame::containing_address(x86_64::VirtAddr::from_u64(current_addr)), 
                                     TABLE_FLAGS.into()).is_err() {
            panic!("Failed to initialize kernel identity mapping!");
        }

        current_addr += Size4KiB::SIZE;
    }
    
    // Create a page table for the PML4 (level 1) and map it
    let pml4_frame = pager_manager.mapper.allocate_frames(PhysFrame::containing_address(
        x86_64::VirtAddr::from_u64(0x3_0000)
    ), Size4KiB).unwrap();

    // Map the PML4 frame to itself (recursive mapping) at address 0x1_0000
    pager_manager.mapper.map_to(
        x86_64::structures::paging::Page::<Size4KiB>::containing_address(x86_64::VirtAddr::from_u64(0x1_0000)),
        pml4_frame,
        TABLE_FLAGS.into()
    );

    // Set up the PML4 register to point to our page table
    unsafe {
        let cr3 = Cr3::read();
        let mut new_cr3 = cr3;
        
        // Update CR3 with the frame containing the PML4 table and flags
        new_cr3.set_pml4_frame(pml4_frame);
        Cr3::write(new_cr3.frame(), new_cr3.flags());
    }

    pager_manager
}
