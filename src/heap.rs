use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};
use spin::Mutex;

/// Heap configuration constants
const HEAP_SIZE: usize = 64 * 1024; // 64 KiB heap
const MIN_BLOCK_SIZE: usize = 16;   // Minimum allocation size

/// Static heap memory
static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// A node in the free list
#[repr(C)]
struct FreeNode {
    size: usize,
    next: Option<NonNull<FreeNode>>,
}

impl FreeNode {
    /// Create a new free node at the given address
    ///
    /// # Safety
    /// The caller must ensure that the memory at `addr` is valid and properly aligned.
    unsafe fn new_at(addr: *mut u8, size: usize) -> NonNull<FreeNode> {
        let node = addr as *mut FreeNode;
        (*node).size = size;
        (*node).next = None;
        NonNull::new_unchecked(node)
    }
}

/// Linked list heap allocator
pub struct LinkedListAllocator {
    head: Option<NonNull<FreeNode>>,
    initialized: bool,
}

// SAFETY: The allocator is protected by a Mutex, ensuring single-threaded access
unsafe impl Send for LinkedListAllocator {}

impl LinkedListAllocator {
    /// Create a new uninitialized allocator
    pub const fn new() -> Self {
        LinkedListAllocator {
            head: None,
            initialized: false,
        }
    }

    /// Initialize the allocator with the static heap memory
    ///
    /// # Safety
    /// This must only be called once before any allocations.
    pub unsafe fn init(&mut self) {
        if self.initialized {
            return;
        }

        let heap_start = core::ptr::addr_of_mut!(HEAP_MEMORY) as *mut u8;
        let initial_node = FreeNode::new_at(heap_start, HEAP_SIZE);
        self.head = Some(initial_node);
        self.initialized = true;
    }

    /// Align the given address upwards to the specified alignment
    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }

    /// Allocate a block of memory
    unsafe fn alloc_impl(&mut self, layout: Layout) -> *mut u8 {
        if !self.initialized {
            return ptr::null_mut();
        }

        let size = layout.size().max(MIN_BLOCK_SIZE);
        let align = layout.align().max(core::mem::align_of::<FreeNode>());

        // Required size includes space for storing the block size
        let total_size = size + core::mem::size_of::<usize>();

        let mut prev: Option<NonNull<FreeNode>> = None;
        let mut current = self.head;

        while let Some(node) = current {
            let node_ref = node.as_ref();
            let node_addr = node.as_ptr() as usize;

            // Calculate aligned start address for user data (after size header)
            let data_start = node_addr + core::mem::size_of::<usize>();
            let aligned_start = Self::align_up(data_start, align);
            let header_offset = aligned_start - core::mem::size_of::<usize>() - node_addr;
            let required_size = header_offset + total_size;

            if node_ref.size >= required_size {
                // Found a suitable block
                let next = node_ref.next;

                // Calculate remaining space after allocation
                let remaining = node_ref.size - required_size;

                if remaining >= core::mem::size_of::<FreeNode>() + MIN_BLOCK_SIZE {
                    // Split the block: create a new free node after our allocation
                    let new_node_addr = (node_addr + required_size) as *mut u8;
                    let new_node = FreeNode::new_at(new_node_addr, remaining);
                    (*new_node.as_ptr()).next = next;

                    // Update the linked list
                    match prev {
                        Some(mut p) => p.as_mut().next = Some(new_node),
                        None => self.head = Some(new_node),
                    }
                } else {
                    // Use the entire block
                    match prev {
                        Some(mut p) => p.as_mut().next = next,
                        None => self.head = next,
                    }
                }

                // Store the allocated size at the header location
                let header_addr = (aligned_start - core::mem::size_of::<usize>()) as *mut usize;
                *header_addr = required_size;

                return aligned_start as *mut u8;
            }

            prev = current;
            current = node_ref.next;
        }

        // No suitable block found
        ptr::null_mut()
    }

    /// Free a previously allocated block
    unsafe fn dealloc_impl(&mut self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() || !self.initialized {
            return;
        }

        // Read the block size from the header
        let header_addr = (ptr as usize - core::mem::size_of::<usize>()) as *mut usize;
        let block_size = *header_addr;

        // Find the actual start of the block by searching backwards
        // The header is directly before ptr, but the block may have started earlier due to alignment
        let block_start = header_addr as *mut u8;

        // Create a new free node at this location
        let new_node = FreeNode::new_at(block_start, block_size);

        // Insert into the free list in address order for coalescing
        let mut prev: Option<NonNull<FreeNode>> = None;
        let mut current = self.head;

        while let Some(node) = current {
            if (node.as_ptr() as usize) > (new_node.as_ptr() as usize) {
                break;
            }
            prev = current;
            current = node.as_ref().next;
        }

        // Insert the new node
        (*new_node.as_ptr()).next = current;
        match prev {
            Some(mut p) => p.as_mut().next = Some(new_node),
            None => self.head = Some(new_node),
        }

        // Coalesce with next block if adjacent
        self.coalesce_next(new_node);

        // Coalesce with previous block if adjacent
        if let Some(p) = prev {
            self.coalesce_next(p);
        }
    }

    /// Coalesce a node with its next neighbor if they are adjacent
    unsafe fn coalesce_next(&mut self, node: NonNull<FreeNode>) {
        let node_ref = node.as_ptr();
        if let Some(next) = (*node_ref).next {
            let node_end = (node_ref as usize) + (*node_ref).size;
            let next_start = next.as_ptr() as usize;

            if node_end == next_start {
                // Merge the blocks
                (*node_ref).size += (*next.as_ptr()).size;
                (*node_ref).next = (*next.as_ptr()).next;
            }
        }
    }
}

/// Thread-safe wrapper for the allocator
pub struct LockedAllocator(Mutex<LinkedListAllocator>);

impl LockedAllocator {
    pub const fn new() -> Self {
        LockedAllocator(Mutex::new(LinkedListAllocator::new()))
    }

    /// Initialize the allocator
    ///
    /// # Safety
    /// Must be called exactly once before any allocations.
    pub unsafe fn init(&self) {
        self.0.lock().init();
    }
}

unsafe impl GlobalAlloc for LockedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.lock().alloc_impl(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc_impl(ptr, layout);
    }
}

/// The global allocator instance
#[global_allocator]
pub static ALLOCATOR: LockedAllocator = LockedAllocator::new();

/// Initialize the heap allocator
///
/// # Safety
/// Must be called exactly once before using heap allocations.
pub unsafe fn init_heap() {
    ALLOCATOR.init();
}
