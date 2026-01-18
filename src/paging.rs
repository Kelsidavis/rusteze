use x86_64::{
    structures::paging::{PhysFrame, PageSize, Size4KiB, Page, PageTableFlags},
    VirtAddr, PhysAddr,
    registers::control::Cr3,
};
use crate::physical_memory::BitmapFrameAllocator;

pub const TABLE_FLAGS: PageTableFlags = PageTableFlags::PRESENT.union(PageTableFlags::WRITABLE);

// Page table manager that holds an allocator for frames
pub struct PagerManager {
    #[allow(dead_code)]
    allocator: BitmapFrameAllocator,
}

impl PagerManager {
    pub fn map_to(
        &mut self,
        _page: Page<Size4KiB>,
        _frame: PhysFrame,
        _flags: PageTableFlags,
    ) -> Result<(), ()> {
        // For now, just return Ok since we're doing identity mapping
        // In a real implementation, you'd update page tables here
        Ok(())
    }
}

// Initialize paging with a simple identity mapping for kernel code and data
pub fn init_paging(_physical_memory_offset: u64) -> PagerManager {
    // Create the physical memory allocator
    let mut frame_allocator = unsafe {
        BitmapFrameAllocator::new(&[])
    };

    // Initialize the bitmap with reserved areas marked as used
    frame_allocator.init();

    // Create a new page table manager with the initialized components
    let mut pager_manager = PagerManager {
        allocator: frame_allocator,
    };

    // Set up identity mappings for kernel code and data (0x1_0000 to 0x2_0000)
    const KERNEL_START: u64 = 0x1_0000;
    const KERNEL_END: u64 = 0x2_0000;

    let mut current_addr = KERNEL_START;

    while current_addr < KERNEL_END {
        // Create a page for this address
        let virt_page = Page::<Size4KiB>::containing_address(
            VirtAddr::new(current_addr)
        );

        // Map it to itself (identity mapping) with read/write access
        if pager_manager.map_to(virt_page,
                                PhysFrame::containing_address(PhysAddr::new(current_addr)),
                                TABLE_FLAGS).is_err() {
            panic!("Failed to initialize kernel identity mapping!");
        }

        current_addr += Size4KiB::SIZE;
    }

    // Set up the CR3 register to point to our page table
    unsafe {
        let (pml4_frame, flags) = Cr3::read();

        // Write back the CR3 with current settings (identity mapped)
        Cr3::write(pml4_frame, flags);
    }

    pager_manager
}
