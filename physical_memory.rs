// src/physical_memory.rs
//! Physical memory management using a bitmap allocator

use crate::gdt;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    VirtAddr,
};
use bootloader_api::bootinfo::MemoryMap;

/// A frame allocator that uses a bitmap to track allocated frames
pub struct BitmapFrameAllocator {
    memory_map: MemoryMap,
    bitmaps: Vec<u8>,
}

impl BitmapFrameAllocator {
    /// Creates a new bitmap frame allocator from the provided memory map.
    pub const fn new(memory_map: MemoryMap) -> Self {
        // Calculate how many bits we need to track all frames
        let total_frames = calculate_total_frame_count(&memory_map);
        
        // Each byte holds 8 bits, so divide by 8 and round up
        let bitmap_size = (total_frames + 7) / 8;
        
        BitmapFrameAllocator {
            memory_map,
            bitmaps: vec![0; bitmap_size],
        }
    }

    /// Calculates the total number of frames needed based on available RAM.
    fn calculate_total_frame_count(memory_map: &MemoryMap) -> usize {
        let mut count = 0;
        for region in memory_map.iter() {
            // Only consider usable regions (not reserved, ACPI tables, etc.)
            if region.region_type == bootloader_api::bootinfo::RegionType::Usable {
                // Convert bytes to frames
                let frame_count = region.range.end - region.range.start + 1;
                count += frame_count / Size4KiB::SIZE as u64;
            }
        }
        
        count as usize
    }

    /// Marks a physical frame as allocated in the bitmap.
    fn mark_frame_as_allocated(&mut self, frame: PhysFrame) {
        let index = self.frame_to_index(frame);
        if index < self.bitmaps.len() * 8 {
            // Set bit to 1 (allocated)
            self.bitmaps[index / 8] |= 1 << (index % 8);
        }
    }

    /// Marks a physical frame as free in the bitmap.
    fn mark_frame_as_free(&mut self, frame: PhysFrame) {
        let index = self.frame_to_index(frame);
        if index < self.bitmaps.len() * 8 {
            // Set bit to 0 (free)
            self.bitmaps[index / 8] &= !(1 << (index % 8));
        }
    }

    /// Converts a physical frame into its corresponding bitmap index.
    fn frame_to_index(&self, frame: PhysFrame) -> usize {
        let start_addr = VirtAddr::new(self.memory_map[0].range.start);
        
        // Calculate the offset of this frame from the first usable region
        (frame.start_address().as_u64() - self.memory_map[0].range.start)
            / Size4KiB::SIZE as u64 as usize
    }

    /// Returns true if a physical frame is allocated.
    fn is_frame_allocated(&self, frame: PhysFrame) -> bool {
        let index = self.frame_to_index(frame);
        
        // Check the corresponding bit in our bitmap array
        (index < self.bitmaps.len() * 8)
            && ((self.bitmaps[index / 8] >> (index % 8)) & 1 == 1)
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    /// Allocates a physical frame.
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Find the first free bit in our bitmap
        let mut index = 0;
        
        while index < self.bitmaps.len() * 8 {
            if (self.bitmaps[index / 8] >> (index % 8)) & 1 == 0 {
                // Found a free frame, mark it as allocated and return it
                
                // Calculate the physical address of this frame
                let start_addr = VirtAddr::new(self.memory_map[0].range.start);
                
                // Convert index back to actual physical address in bytes
                let offset_bytes = (index * Size4KiB::SIZE) as u64;
                let phys_frame_start = self.memory_map[0].range.start + offset_bytes;

                if !self.is_valid_physical_address(phys_frame_start) {
                    return None; // Invalid frame, skip it
                }

                let frame = PhysFrame::from_start_addr(
                    unsafe { x86_64::PhysAddr::new_unchecked(phys_frame_start as u64) },
                    Size4KiB,
                ).unwrap_or_else(|_| panic!("Invalid physical address"));

                // Mark this frame as allocated
                self.mark_frame_as_allocated(frame);
                
                return Some(frame);
            }
            
            index += 1;
        }

        None // No free frames available
    }
}

impl BitmapFrameAllocator {
    /// Checks if a given physical address is valid and within usable memory regions.
    fn is_valid_physical_address(&self, addr: u64) -> bool {
        for region in self.memory_map.iter() {
            if region.region_type == bootloader_api::bootinfo::RegionType::Usable &&
               addr >= region.range.start && 
               addr < region.range.end + 1
            {
                return true;
            }
        }
        
        false
    }

    /// Initializes the allocator by marking all frames as free.
    pub fn init(&mut self) {
        // Clear all bits (mark everything as free)
        for byte in &mut self.bitmaps {
            *byte = 0;
        }
        
        // Mark reserved regions and boot information areas as allocated
        let mut used_frames: Vec<u64> = vec![];
        
        // Add the kernel's own memory region to our list of "used" frames
        for region in &self.memory_map {
            if region.region_type == bootloader_api::bootinfo::RegionType::Usable &&
               (region.range.start as u64) < 0x1_0000 && // Below 1MB, likely reserved by BIOS/UEFI
                !used_frames.contains(&(region.range.start))
            {
                used_frames.push(region.range.start);
                
                let frame_count = region.range.end - region.range.start + 1;
                for i in 0..(frame_count / Size4KiB::SIZE as u64) {
                    if (i * Size4KiB::SIZE as u64) < 256_000 { // Only mark frames below 256KB
                        let frame_addr = region.range.start + i * Size4KiB::SIZE;
                        
                        self.mark_frame_as_allocated(
                            PhysFrame::from_start_addr(
                                unsafe {
                                    x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                                },
                                Size4KiB,
                            ).unwrap_or_else(|_| panic!("Invalid frame address")),
                        );
                    }
                }
            }
        }

        // Mark the GDT and IDT areas (if they're in usable memory) 
        let gdt_start = unsafe { &gdt::GDT as *const _ } as u64;
        
        if self.is_valid_physical_address(gdt_start) {
            for i in 0..(128 / Size4KiB::SIZE + 1) {
                let frame_addr = (gdt_start - gdt_start % Size4KiB::SIZE)
                    + i * Size4KiB::SIZE;
                
                if self.is_valid_physical_address(frame_addr) && 
                   !self.is_frame_allocated(
                       PhysFrame::from_start_addr(
                           unsafe { x86_64::PhysAddr::new_unchecked(frame_addr as u64) },
                           Size4KiB,
                       ).unwrap_or_else(|_| panic!("Invalid frame address"))
                   ) {
                    self.mark_frame_as_allocated(
                        PhysFrame::from_start_addr(
                            unsafe { 
                                x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                            }, 
                            Size4KiB,
                        ).unwrap_or_else(|_| panic!("Invalid frame address"))
                    );
                }
            }
        }

        // Mark the IDT area
        let idt_start = unsafe { &idt::IDT as *const _ } as u64;
        
        if self.is_valid_physical_address(idt_start) {
            for i in 0..(256 / Size4KiB::SIZE + 1) {
                let frame_addr = (idt_start - idt_start % Size4KiB::SIZE)
                    + i * Size4KiB::SIZE;
                
                if self.is_valid_physical_address(frame_addr) && 
                   !self.is_frame_allocated(
                       PhysFrame::from_start_addr(
                           unsafe { x86_64::PhysAddr::new_unchecked(frame_addr as u64) },
                           Size4KiB,
                       ).unwrap_or_else(|_| panic!("Invalid frame address"))
                   ) {
                    self.mark_frame_as_allocated(
                        PhysFrame::from_start_addr(
                            unsafe { 
                                x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                            }, 
                            Size4KiB,
                        ).unwrap_or_else(|_| panic!("Invalid frame address"))
                    );
                }
            }
        }

        // Mark the VGA buffer area
        let vga_start = 0xB8000;
        
        if self.is_valid_physical_address(vga_start) {
            for i in 0..(4_096 / Size4KiB::SIZE + 1) { 
                let frame_addr = (vga_start - vga_start % Size4KiB::SIZE)
                    + i * Size4KiB::SIZE;
                
                if self.is_valid_physical_address(frame_addr) && 
                   !self.is_frame_allocated(
                       PhysFrame::from_start_addr(
                           unsafe {
                               x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                           },
                           Size4KiB,
                       ).unwrap_or_else(|_| panic!("Invalid frame address"))
                   ) {
                    self.mark_frame_as_allocated(
                        PhysFrame::from_start_addr(
                            unsafe { 
                                x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                            }, 
                            Size4KiB,
                        ).unwrap_or_else(|_| panic!("Invalid frame address"))
                    );
                }
            }
        }

        // Mark the serial port I/O region
        let com1_start = 0x3F8;
        
        if self.is_valid_physical_address(com1_start) {
            for i in 0..(256 / Size4KiB::SIZE + 1) { 
                let frame_addr = (com1_start - com1_start % Size4KiB::SIZE)
                    + i * Size4KiB::SIZE;
                
                if self.is_valid_physical_address(frame_addr) && 
                   !self.is_frame_allocated(
                       PhysFrame::from_start_addr(
                           unsafe {
                               x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                           },
                           Size4KiB,
                       ).unwrap_or_else(|_| panic!("Invalid frame address"))
                   ) {
                    self.mark_frame_as_allocated(
                        PhysFrame::from_start_addr(
                            unsafe { 
                                x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                            }, 
                            Size4KiB,
                        ).unwrap_or_else(|_| panic!("Invalid frame address"))
                    );
                }
            }
        }

        // Mark the PIT timer region
        let pit_start = 0x40;
        
        if self.is_valid_physical_address(pit_start) {
            for i in 0..(256 / Size4KiB::SIZE + 1) { 
                let frame_addr = (pit_start - pit_start % Size4KiB::SIZE)
                    + i * Size4KiB::SIZE;
                
                if self.is_valid_physical_address(frame_addr) && 
                   !self.is_frame_allocated(
                       PhysFrame::from_start_addr(
                           unsafe {
                               x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                           },
                           Size4KiB,
                       ).unwrap_or_else(|_| panic!("Invalid frame address"))
                   ) {
                    self.mark_frame_as_allocated(
                        PhysFrame::from_start_addr(
                            unsafe { 
                                x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                            }, 
                            Size4KiB,
                        ).unwrap_or_else(|_| panic!("Invalid frame address"))
                    );
                }
            }
        }

        // Mark the bootloader's memory region
        let boot_info_start = 0x9FC00;
        
        if self.is_valid_physical_address(boot_info_start) {
            for i in 0..(256 / Size4KiB::SIZE + 1) { 
                let frame_addr = (boot_info_start - boot_info_start % Size4KiB::SIZE)
                    + i * Size4KiB::SIZE;
                
                if self.is_valid_physical_address(frame_addr) && 
                   !self.is_frame_allocated(
                       PhysFrame::from_start_addr(
                           unsafe {
                               x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                           },
                           Size4KiB,
                       ).unwrap_or_else(|_| panic!("Invalid frame address"))
                   ) {
                    self.mark_frame_as_allocated(
                        PhysFrame::from_start_addr(
                            unsafe { 
                                x86_64::PhysAddr::new_unchecked(frame_addr as u64)
                            }, 
                            Size4KiB,
                        ).unwrap_or_else(|_| panic!("Invalid frame address"))
                    );
                }
            }
        }

    }
}

/// Calculates the total number of frames needed based on available RAM.
fn calculate_total_frame_count(memory_map: &MemoryMap) -> usize {
    let mut count = 0;
    
    for region in memory_map.iter() {
        if region.region_type == bootloader_api::bootinfo::RegionType::Usable {
            // Convert bytes to frames
            let frame_count = (region.range.end - region.range.start + 1)
                / Size4KiB::SIZE as u64;
            
            count += frame_count as usize;
        }
    }

    count
}
