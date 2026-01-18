// This module implements 4-level page tables (PML4 -> PDPT -> PDT -> PT)
// Each level has 512 entries of 8 bytes each = 4KB per level

use crate::physical_memory;
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, PhysFrame, Size4KiB},
    VirtAddr,
};
use x86_64::structures::paging::mapper::MapToError;

// Page table entry flags
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const USER_ACCESSIBLE: u64 = 1 << 2;
const WRITE_THROUGH: u64 = 1 << 3;
const CACHE_DISABLED: u64 = 1 << 4;

// Page table entry flags for page tables
pub const TABLE_FLAGS: x86_64::structures::paging::PageTableFlags =
    x86_64::structures::paging::PageTableFlags::PRESENT |
    x86_64::structures::paging::PageTableFlags::WRITABLE;

// Frame allocator that uses the boot info memory regions
pub struct BootInfoFrameAllocator {
    frame_allocator: physical_memory::BitmapFrameAllocator,
}

impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Use the boot info allocator to get a 4KB frame
        let phys_addr = self.frame_allocator.allocate()?;
        
        Some(phys_addr)
    }
}

// Page table manager that handles mapping virtual addresses to physical frames
pub struct PageTableManager {
    mapper: Mapper<'static, Size4KiB>,
    frame_allocator: BootInfoFrameAllocator,
}

impl PageTableManager {
    pub fn new(
        physical_memory_offset: u64,
        frame_allocator: &mut physical_memory::BitmapFrameAllocator,
    ) -> Self {
        // Create a mapping from virtual to physical addresses
        let mapper = unsafe { 
            Mapper::new(VirtAddr::from_u64(physical_memory_offset), false)
        };

        PageTableManager {
            mapper,
            frame_allocator: BootInfoFrameAllocator {
                frame_allocator: frame_allocator.clone(),
            },
        }
    }

    // Map a virtual address to physical memory
    pub fn map_to(
        &mut self,
        page: x86_64::structures::paging::Page<Size4KiB>,
        phys_frame: PhysFrame,
        flags: u64,
    ) -> Result<(), MapToError> {
        // Ensure the mapping is valid and not already mapped
        let map_result = unsafe { 
            self.mapper.map_to(
                page, 
                phys_frame, 
                x86_64::structures::paging::MapFlags::from_bits_retain(flags),
                &mut self.frame_allocator,
            )
        };

        // Flush the TLB entry for this mapping
        map_result.and_then(|_| unsafe {
            use x86_64::instructions::tlb;
            tlb::flush(page.start_address());
        })
    }

    pub fn create_page_table(&self) -> PhysFrame {
        let frame = self.frame_allocator.allocate_frame().unwrap();
        
        // Zero out the page table
        unsafe { 
            core::ptr::write_bytes(
                frame.start_address().as_ptr() as *mut u8,
                0, 
                Size4KiB::SIZE as usize
            );
        }

        frame
    }
}

// Initialize paging with a simple identity mapping for kernel code and data
pub fn init_paging(physical_memory_offset: u64) -> PageTableManager {
    // Create the page table manager using boot info allocator
    let mut frame_allocator = unsafe { 
        physical_memory::BitmapFrameAllocator::new(crate::BOOT_INFO.memory_regions.as_ref())
    };
    
    frame_allocator.init();

    let mut pager_manager = PageTableManager::new(physical_memory_offset, &mut frame_allocator);

    // Create identity mappings for kernel code and data
    const KERNEL_START: u64 = 0x1_0000;
    const KERNEL_END: u64 = 0x2_0000;

    let mut current_addr = KERNEL_START;
    
    while current_addr < KERNEL_END {
        // Create a page for this address
        let virt_page = x86_64::structures::paging::Page::<Size4KiB>::containing_address(
            VirtAddr::from_u64(current_addr)
        );

        // Allocate physical frame and map it to virtual address with read/write access
        pager_manager.map_to(virt_page, 
                             PhysFrame::containing_address(VirtAddr::from_u64(current_addr)), 
                             TABLE_FLAGS.bits());

        current_addr += Size4KiB::SIZE;
    }

    // Map the page table itself (for recursive mapping)
    let pml4_frame = pager_manager.create_page_table();
    
    // Create a virtual address for PML4
    let mut virt_pml4_address = VirtAddr::from_u64(0x1_0000);
    while !virt_pml4_address.as_u64().is_aligned_to(Size4KiB::SIZE) {
        virt_pml4_address += Size4KiB::SIZE;
    }

    // Map the PML4 frame to itself (recursive mapping)
    pager_manager.map_to(
        x86_64::structures::paging::Page::<Size4KiB>::containing_address(virt_pml4_address),
        pml4_frame,
        TABLE_FLAGS.bits()
    );

    // Set up the PML4 register to point to our page table
    unsafe {
        use x86_64::registers::control::{Cr3, Cr3Flags};
        
        let cr3 = Cr3::read();
        let mut new_cr3 = cr3;
        new_cr3.set_pml4_frame(pml4_frame);
        // Fix: Call write with separate arguments
        Cr3::write(new_cr3.frame(), new_cr3.flags());
    }

    pager_manager
}
