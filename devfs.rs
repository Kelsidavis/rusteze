// src/devfs.rs

use crate::vfs::{Inode, File};
use core::cell::UnsafeCell;
use x86_64::{
    structures::gdt,
    registers::segmentation::SegmentSelector
};

/// Device file types.
#[derive(Debug, Clone, Copy)]
pub enum DevFileType {
    /// Null device - discards all data written to it and returns EOF on reads
    Null,

    /// Zero device - provides infinite stream of zero bytes (0x00)
    Zero,
    
    /// Console terminal for text I/O with the user interface.
    Console,

    /// Random number generator device 
    Random,
}

/// A devfs inode that represents a special file in /dev.
pub struct DevInode {
    inner: UnsafeCell<DevFile>,
    
    /// The device name (e.g., "null", "zero")
    pub name: &'static str,

    // We'll add more fields as needed for different devices
}

impl DevInode {
    /// Creates a new devfs inode with the given type and name.
    ///
    /// # Arguments
    /// * `dev_type` - The device file type (Null, Zero, Console)
    /// * `name` - The filename of this special device file
    pub const fn new(dev_type: DevFileType, name: &'static str) -> Self {
        Self { 
            inner: UnsafeCell::new(DevFile::default()),
            name,
        }
    }

    /// Gets the type associated with this devfs inode.
    #[inline]
    pub fn get_dev_type(&self) -> DevFileType {
        unsafe { (*self.inner.get()).get_dev_type() } 
    }
}

/// A device file that represents a special hardware resource
pub struct DevFile {
    dev_type: DevFileType,
    
    // Count of open instances for this device (used to prevent deletion)
    pub open_count: AtomicU32,

    /// Current position in the stream when reading/writing.
    pos: u64,

    /// Whether we're currently processing a read operation
    is_reading: bool,
}

impl DevFile {
    /// Creates a new device file with the given type and initial state.
    pub const fn default() -> Self {
        Self { 
            dev_type: DevFileType::Null,  // Placeholder value - will be set by constructor
            open_count: AtomicU32::new(0),
            pos: 0,
            is_reading: false,
        }
    }

    /// Creates a new device file with the given type and initial state.
    pub const fn new(dev_type: DevFileType) -> Self {
        Self { 
            dev_type, 
            open_count: AtomicU32::new(0),
            pos: 0,
            is_reading: false
        }
    }

    /// Gets the device file's type (Null/Zero/Console/etc.)
    #[inline]
    pub fn get_dev_type(&self) -> DevFileType {
        self.dev_type
    }

    // Device-specific operations:

    /// Reads from a special device like /dev/null or /dev/random.
    ///
    /// # Arguments
    /// * `buf` - Buffer to write data into (up to buf.len() bytes)
    pub fn dev_read(&mut self, buf: &mut [u8]) -> usize {
        match self.dev_type {
            DevFileType::Null => { 
                // Null device returns EOF on all reads after first read
                return 0;
            },
            
            DevFileType::Zero => { 
                let len = core::cmp::min(buf.len(), 128); // Read up to 128 bytes at once
                
                for i in 0..len {
                    buf[i] = 0x00; // Fill with zero byte
                }
                
                self.pos += len as u64;
                return len;
            },
            
            DevFileType::Console => { 
                let mut count: usize = 0;

                while count < buf.len() && !self.is_input_buffer_empty() {
                    if ProcessControlBlock::is_input_buffer_empty() {
                        break; // No more input available yet
                    }
                    
                    buf[count] = ProcessControlBlock::get_next_char_from_input();
                    self.pos += 1;
                    count += 1;

                    // Limit to reasonable number of chars per read call for safety and responsiveness
                    if (count + 1) % 8 == 0 {
                        break; 
                    }
                }

                return count;
            },

            DevFileType::Random => { 
                let len = core::cmp::min(buf.len(), 256); // Read up to 256 bytes at once
                
                for i in 0..len {
                    buf[i] = get_random_byte();
                    
                    self.pos += 1;
                }
                
                return len;  
            },
        };
    }

    /// Writes to a special device like /dev/null or /dev/zero.
    ///
    /// # Arguments
    /// * `buf` - Data buffer containing bytes to write (up to buf.len() bytes)
    pub fn dev_write(&mut self, buf: &[u8]) -> usize {
        match self.dev_type {
            DevFileType::Null => { 
                // Null device discards all writes silently  
                return 0;
            },
            
            DevFileType::Zero | DevFileType::Console => {
                panic!("Cannot write to /dev/zero or /dev/console");
            },

            _ => unreachable!(),
        }
    }

    /// Gets the current position in this device stream.
    #[inline]
    pub fn get_pos(&self) -> u64 {
        self.pos
    }

    /// Sets the current position in this device stream (seek).
    #[inline]
    pub fn set_pos(&mut self, pos: u64) {
        // Limit to reasonable values for safety and consistency with other devices
        if pos > 1024 * 1024 { 
            panic!("Position too large"); 
        }
        
        self.pos = pos;
    }

    /// Gets the current open count (number of active file descriptors).
    #[inline]
    pub fn get_open_count(&self) -> u32 {
        self.open_count.load(Ordering::Relaxed)
    }

    // Helper methods for device-specific operations:
    
    /// Increments the open counter.
    #[inline]
    pub fn inc_open_count(&mut self) {
        let old = self.open_count.fetch_add(1, Ordering::SeqCst);
        
        if old == 0 && matches!(self.dev_type, DevFileType::Null | DevFileType::Zero)) { 
            // First opener of null/zero - ensure we don't allow deletion
            panic!("Cannot delete /dev/null or /dev/zero while open");
        }
    }

    /// Decrements the open counter.
    #[inline]
    pub fn dec_open_count(&mut self) {
        let old = self.open_count.fetch_sub(1, Ordering::SeqCst);
        
        if old == 0 { 
            panic!("Open count underflow"); // Shouldn't happen
        } else if old == 1 && matches!(self.dev_type, DevFileType::Null | DevFileType::Zero)) {
            // Last opener of null/zero - allow deletion now (though we don't actually delete)
            self.pos = u64::MAX; 
        }
    }

    /// Sets the reading flag.
    #[inline]
    pub fn set_reading(&mut self) {
        self.is_reading = true;
    }

    /// Clears the reading flag.
    #[inline]
    pub fn clear_reading(&mut self) {
        self.is_reading = false;
    }
}

// Implementations of the Inode trait
impl crate::vfs::Inode for DevInode {
    // Implementation details...
}
