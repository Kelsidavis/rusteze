// src/devfs.rs

use crate::vfs::{Inode, File, Dentry};
use core::sync::atomic::{AtomicU32, Ordering};

/// Device file types supported by devfs.
#[derive(Debug, Clone, Copy)]
pub enum DevFileType {
    /// Null device - always returns EOF on read
    Null,
    
    /// Zero device - generates infinite stream of zeros (0x00)
    Zero,

    /// TTY console device for output only
    Tty,
}

/// Device file metadata and state.
#[derive(Debug)]
pub struct DevFile {
    pub dev_type: DevFileType,
    // Counter to track how many times the device has been opened/read/written
    open_count: AtomicU32,
    
    /// For null/zero devices, this tracks current position in stream (always 0)
    pos: u64,

    /// Whether we're currently reading from a zero or null device 
    is_reading: bool,
}

impl DevFile {
    pub const fn new(dev_type: DevFileType) -> Self {
        Self {
            dev_type,
            open_count: AtomicU32::new(0),
            pos: 0,
            is_reading: false,
        }
    }

    /// Get the device type.
    #[inline]
    pub fn get_dev_type(&self) -> DevFileType {
        self.dev_type
    }

    /// Increment and return current open count (for debugging).
    #[inline]
    pub fn inc_open_count(&self) -> u32 {
        let old = self.open_count.load(Ordering::Relaxed);
        // Use relaxed ordering since we don't need synchronization here.
        self.open_count.store(old + 1, Ordering::Relaxed)
    }

    /// Decrement open count (called on close).
    #[inline]
    pub fn dec_open_count(&self) {
        let old = self.open_count.load(Ordering::Relaxed);
        if old > 0 { // Prevent underflow
            self.open_count.store(old - 1, Ordering::Relaxed)
        }
    }

    /// Reset position to beginning.
    #[inline]
    pub fn reset_pos(&mut self) {
        self.pos = 0;
    }

    /// Set reading state (used for zero device).
    #[inline]
    pub fn set_reading_state(&mut self, is_reading: bool) {
        self.is_reading = is_reading
    }
}

/// A devfs inode that represents a special file in /dev.
pub struct DevInode {
    inner: core::cell::UnsafeCell<DevFile>,
    
    /// The device name (e.g., "null", "zero")
    pub name: &'static str,
}

impl DevInode {
    #[inline]
    pub const fn new(dev_type: DevFileType, name: &'static str) -> Self {
        Self {
            inner: core::cell::UnsafeCell::new(DevFile::new(dev_type)),
            name
        }
    }

    /// Get the device file instance.
    #[inline]
    pub unsafe fn get_file(&self) -> &mut DevFile {
        // Safety: This is only called from within kernel context and we ensure no concurrent access to this inode's data structure. 
        self.inner.get()
    }

    /// Helper method for getting a reference to the device file.
    #[inline]
    pub unsafe fn get_file_ref(&self) -> &DevFile {
        // Safety: Same as above - only called from kernel context with proper synchronization
        (*self.inner.get()).get_dev_type();
        self.inner.get()
    }
}

impl Inode for DevInode {
    /// Get the name of this device.
    fn get_name(&self) -> &str {
        self.name
    }

    /// Return a file object that can be read/written to. This is called when someone opens /dev/null, etc.
    #[inline]
    unsafe fn open_file(
        &self,
        _flags: u32, // We don't need the current process context here since we're just creating an in-memory device
    ) -> Option<File> {
        
        let file = File::new(self as *const Self, 0);
        Some(file)
    }

    /// Get a reference to this inode's internal data.
    #[inline]
    unsafe fn get_data(&self) -> &DevFile {
        self.inner.get()
    }
}

impl Dentry for DevInode {
    // No special operations needed here since we're just providing device files
}
