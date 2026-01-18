use bootloader_api::info::{MemoryRegion, MemoryRegionKind};

/// Size of a physical frame (4 KiB)
pub const FRAME_SIZE: usize = 4096;

/// Maximum physical memory we support (1 GiB = 262144 frames)
const MAX_FRAMES: usize = 262144;

/// Bitmap size in bytes (1 bit per frame)
const BITMAP_SIZE: usize = MAX_FRAMES / 8;

/// Static bitmap storage for frame allocation
/// Each bit represents one 4 KiB frame: 0 = free, 1 = used
static mut BITMAP: [u8; BITMAP_SIZE] = [0xFF; BITMAP_SIZE]; // Start with all used

/// Physical frame number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysFrame(pub u64);

impl PhysFrame {
    /// Create a frame from a physical address (must be 4K aligned)
    #[allow(dead_code)]
    pub fn containing_address(addr: u64) -> Self {
        PhysFrame(addr / FRAME_SIZE as u64)
    }

    /// Get the start address of this frame
    #[allow(dead_code)]
    pub fn start_address(&self) -> u64 {
        self.0 * FRAME_SIZE as u64
    }
}

/// Bitmap-based physical frame allocator
pub struct BitmapFrameAllocator {
    /// Memory regions from bootloader
    memory_regions: &'static [MemoryRegion],
    /// Next frame to check for allocation
    #[allow(dead_code)]
    next_frame: usize,
}

impl BitmapFrameAllocator {
    /// Create a new bitmap frame allocator from bootloader memory regions
    ///
    /// # Safety
    /// This function must only be called once with valid memory regions from the bootloader.
    pub unsafe fn new(memory_regions: &'static [MemoryRegion]) -> Self {
        BitmapFrameAllocator {
            memory_regions,
            next_frame: 0,
        }
    }

    /// Initialize the bitmap by marking usable regions as free
    pub fn init(&mut self) {
        // First, all frames are marked as used (bitmap initialized to 0xFF)
        // Now mark usable regions as free
        for region in self.memory_regions.iter() {
            if region.kind == MemoryRegionKind::Usable {
                let start_frame = region.start / FRAME_SIZE as u64;
                let end_frame = region.end / FRAME_SIZE as u64;

                for frame in start_frame..end_frame {
                    if (frame as usize) < MAX_FRAMES {
                        self.mark_frame_free(frame as usize);
                    }
                }
            }
        }

        crate::serial_println!("Physical memory bitmap initialized");
    }

    /// Mark a frame as free (bit = 0)
    fn mark_frame_free(&mut self, frame: usize) {
        if frame < MAX_FRAMES {
            let byte_idx = frame / 8;
            let bit_idx = frame % 8;
            unsafe {
                BITMAP[byte_idx] &= !(1 << bit_idx);
            }
        }
    }

    /// Mark a frame as used (bit = 1)
    #[allow(dead_code)]
    fn mark_frame_used(&mut self, frame: usize) {
        if frame < MAX_FRAMES {
            let byte_idx = frame / 8;
            let bit_idx = frame % 8;
            unsafe {
                BITMAP[byte_idx] |= 1 << bit_idx;
            }
        }
    }

    /// Check if a frame is free
    #[allow(dead_code)]
    fn is_frame_free(&self, frame: usize) -> bool {
        if frame >= MAX_FRAMES {
            return false;
        }
        let byte_idx = frame / 8;
        let bit_idx = frame % 8;
        unsafe { (BITMAP[byte_idx] & (1 << bit_idx)) == 0 }
    }

    /// Allocate a single physical frame
    #[allow(dead_code)]
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Start from next_frame and wrap around
        for offset in 0..MAX_FRAMES {
            let frame = (self.next_frame + offset) % MAX_FRAMES;
            if self.is_frame_free(frame) {
                self.mark_frame_used(frame);
                self.next_frame = (frame + 1) % MAX_FRAMES;
                return Some(PhysFrame(frame as u64));
            }
        }
        None
    }

    /// Free a previously allocated frame
    #[allow(dead_code)]
    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        self.mark_frame_free(frame.0 as usize);
    }

    /// Get the total number of free frames
    #[allow(dead_code)]
    pub fn free_frame_count(&self) -> usize {
        let mut count = 0;
        for frame in 0..MAX_FRAMES {
            if self.is_frame_free(frame) {
                count += 1;
            }
        }
        count
    }
}
