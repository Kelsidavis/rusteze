//! Virtual Filesystem (VFS) layer
//!
//! Provides a unified interface for all filesystems through inode abstraction

use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::fmt;
use spin::Mutex;

/// File types supported by the VFS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FileType {
    Regular,
    Directory,
    Device,
    Symlink,
}

/// File open flags
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub append: bool,
    pub create: bool,
    pub truncate: bool,
}

impl OpenFlags {
    #[allow(dead_code)]
    pub const fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            append: false,
            create: false,
            truncate: false,
        }
    }

    #[allow(dead_code)]
    pub const fn write_only() -> Self {
        Self {
            read: false,
            write: true,
            append: false,
            create: false,
            truncate: false,
        }
    }

    pub const fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            append: false,
            create: false,
            truncate: false,
        }
    }
}

/// Inode - represents a file or directory in the filesystem
#[allow(dead_code)]
pub trait Inode: Send + Sync {
    /// Read data from the inode at the given offset
    fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, VfsError>;

    /// Write data to the inode at the given offset
    fn write(&self, offset: usize, buffer: &[u8]) -> Result<usize, VfsError>;

    /// Get the file type
    fn file_type(&self) -> FileType;

    /// Get the file size in bytes
    fn size(&self) -> usize;

    /// Lookup a child entry by name (for directories)
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError>;

    /// Create a new file or directory
    fn create(&self, name: &str, file_type: FileType) -> Result<Arc<dyn Inode>, VfsError>;

    /// List directory entries (for directories)
    fn list(&self) -> Result<Vec<String>, VfsError>;

    /// Truncate the file to the given size
    fn truncate(&self, size: usize) -> Result<(), VfsError>;
}

/// Errors that can occur in the VFS layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum VfsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    PermissionDenied,
    InvalidArgument,
    IoError,
    NotImplemented,
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VfsError::NotFound => write!(f, "File or directory not found"),
            VfsError::AlreadyExists => write!(f, "File or directory already exists"),
            VfsError::NotADirectory => write!(f, "Not a directory"),
            VfsError::IsADirectory => write!(f, "Is a directory"),
            VfsError::PermissionDenied => write!(f, "Permission denied"),
            VfsError::InvalidArgument => write!(f, "Invalid argument"),
            VfsError::IoError => write!(f, "I/O error"),
            VfsError::NotImplemented => write!(f, "Not implemented"),
        }
    }
}

/// File descriptor - represents an open file
pub struct FileDescriptor {
    inode: Arc<dyn Inode>,
    offset: Mutex<usize>,
    flags: OpenFlags,
}

#[allow(dead_code)]
impl FileDescriptor {
    pub fn new(inode: Arc<dyn Inode>, flags: OpenFlags) -> Self {
        Self {
            inode,
            offset: Mutex::new(0),
            flags,
        }
    }

    /// Read from the file at the current offset
    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        if !self.flags.read {
            return Err(VfsError::PermissionDenied);
        }

        let mut offset = self.offset.lock();
        let bytes_read = self.inode.read(*offset, buffer)?;
        *offset += bytes_read;
        Ok(bytes_read)
    }

    /// Write to the file at the current offset
    pub fn write(&self, buffer: &[u8]) -> Result<usize, VfsError> {
        if !self.flags.write {
            return Err(VfsError::PermissionDenied);
        }

        let mut offset = self.offset.lock();
        let bytes_written = self.inode.write(*offset, buffer)?;
        *offset += bytes_written;
        Ok(bytes_written)
    }

    /// Seek to a new position in the file
    pub fn seek(&self, position: usize) -> Result<(), VfsError> {
        *self.offset.lock() = position;
        Ok(())
    }

    /// Get the current file offset
    pub fn tell(&self) -> usize {
        *self.offset.lock()
    }

    /// Get the underlying inode
    pub fn inode(&self) -> &Arc<dyn Inode> {
        &self.inode
    }
}

/// File descriptor table for a process
pub struct FileDescriptorTable {
    descriptors: Mutex<Vec<Option<Arc<FileDescriptor>>>>,
}

#[allow(dead_code)]
impl FileDescriptorTable {
    /// Create a new file descriptor table
    pub fn new() -> Self {
        Self {
            descriptors: Mutex::new(Vec::new()),
        }
    }

    /// Allocate a new file descriptor
    pub fn allocate(&self, fd: Arc<FileDescriptor>) -> Result<usize, VfsError> {
        let mut descriptors = self.descriptors.lock();

        // Try to find an empty slot
        for (i, slot) in descriptors.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(fd);
                return Ok(i);
            }
        }

        // No empty slot, append to the end
        let fd_num = descriptors.len();
        descriptors.push(Some(fd));
        Ok(fd_num)
    }

    /// Get a file descriptor by number
    pub fn get(&self, fd: usize) -> Result<Arc<FileDescriptor>, VfsError> {
        let descriptors = self.descriptors.lock();
        descriptors
            .get(fd)
            .and_then(|slot| slot.clone())
            .ok_or(VfsError::InvalidArgument)
    }

    /// Close a file descriptor
    pub fn close(&self, fd: usize) -> Result<(), VfsError> {
        let mut descriptors = self.descriptors.lock();
        if fd < descriptors.len() {
            descriptors[fd] = None;
            Ok(())
        } else {
            Err(VfsError::InvalidArgument)
        }
    }

    /// Duplicate a file descriptor
    pub fn dup(&self, old_fd: usize) -> Result<usize, VfsError> {
        let fd = self.get(old_fd)?;
        self.allocate(fd)
    }
}

impl Default for FileDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}
