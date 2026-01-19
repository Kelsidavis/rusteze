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

    /// TTY console terminal for text I/O with the user interface.
    Console,
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

    /// Gets the device type associated with this devfs inode.
    #[inline]
    pub fn get_dev_type(&self) -> DevFileType {
        unsafe { (*self.inner.get()).get_dev_type() }
    }
}

/// A devfs file that represents an open instance of a special device.
pub struct DevFile {
    pub dev_type: DevFileType,
    
    /// Number of times this device has been opened (for reference counting)
    open_count: AtomicU32,

    // Current position in the stream
    pos: usize,

    // Whether we're currently reading from it 
    is_reading: bool,
}

impl Default for DevFile {
    fn default() -> Self {
        Self {
            dev_type: DevFileType::Null,  // Placeholder value - will be set by constructor
            open_count: AtomicU32::new(0),
            pos: 0,
            is_reading: false,
        }
    }
}

impl DevFile {
    /// Creates a new device file with the given type and initial state.
    pub const fn new(dev_type: DevFileType) -> Self {
        Self { 
            dev_type, 
            open_count: AtomicU32::new(0),
            pos: 0,
            is_reading: false
        }
    }

    /// Gets the current device file's type (null, zero, console).
    #[inline]
    pub fn get_dev_type(&self) -> DevFileType {
        self.dev_type
    }

    // Device-specific operations

    /// Reads from a special device.
    ///
    /// # Arguments
    /// * `buf` - Buffer to write data into
    /// 
    /// Returns the number of bytes read, or 0 if EOF (null) is reached.
    pub fn dev_read(&self, buf: &mut [u8]) -> usize {
        match self.dev_type {
            DevFileType::Null => { // Null device - always returns EOF after first read
                return 0;
            },
            
            DevFileType::Zero => { 
                let len = core::cmp::min(buf.len(), 128); // Read up to 128 bytes at once
                
                for i in 0..len {
                    buf[i] = 0x00; // Fill with zero byte
                    self.pos += 1;
                }
                
                return len;
            },
            
            DevFileType::Console => { 
                let mut count: usize = 0;

                while count < buf.len() && self.pos < ProcessControlBlock::MAX_INPUT_BUFFER_SIZE {
                    if !ProcessControlBlock::is_input_buffer_empty() {
                        // Read from the input buffer
                        buf[count] = ProcessControlBlock::get_next_char_from_input();
                        
                        count += 1;
                        self.pos += 1; 
                    } else { break }
                }

                return count;
            },
        };
    }


    /// Writes to a special device.
    ///
    /// # Arguments
    /// * `buf` - Data buffer containing bytes to write
    /// 
    /// Returns the number of bytes written, or an error if not supported.
    pub fn dev_write(&self, buf: &[u8]) -> usize {
        match self.dev_type {
            DevFileType::Null => { // Null device discards all writes (no effect)
                return buf.len();
            },
            
            DevFileType::Zero => { 
                // Zero device doesn't accept any data - always returns 0
                return 0;
            },

            DevFileType::Console => { 
                let mut count: usize = 0;

                while count < buf.len() {
                    ProcessControlBlock::add_char_to_output(buf[count]);
                    
                    self.pos += 1; // Increment position for console output
                    
                    if !ProcessControlBlock::is_input_buffer_empty() && (count + 1) % 8 == 0 { 
                        break;
                    }
                
                count += 1;

                return count;
            },
        };
    }

}

// Implementations of the Inode trait
impl crate::vfs::Inode for DevInode {
    /// Gets a reference to this inode's file data.
    fn get_data(&self) -> &DevFile {
        unsafe { (*self.inner.get()).get_dev_type() }
    }

    // Implementation details...
}
