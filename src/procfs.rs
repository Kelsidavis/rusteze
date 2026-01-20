//! Process Filesystem (procfs)
//!
//! Provides virtual files for system and process information like /proc/meminfo

use crate::vfs::{Inode, FileType, VfsError};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::format;
use spin::Mutex;

/// Types of proc files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcFileType {
    MemInfo,  // Memory statistics
    // More can be added: CpuInfo, Uptime, etc.
}

/// A proc file node
pub struct ProcFile {
    file_type: ProcFileType,
    #[allow(dead_code)]
    name: String,
}

impl ProcFile {
    pub fn new(file_type: ProcFileType, name: &str) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            file_type,
            name: String::from(name),
        }))
    }

    fn generate_content(&self) -> String {
        match self.file_type {
            ProcFileType::MemInfo => {
                // TODO: Get actual memory statistics from the memory allocator
                format!(
                    "MemTotal:       {} kB\n\
                     MemFree:        {} kB\n\
                     MemAvailable:   {} kB\n",
                    1024, // Placeholder values
                    512,
                    768
                )
            }
        }
    }
}

impl Inode for Mutex<ProcFile> {
    fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let proc_file = self.lock();
        let content = proc_file.generate_content();
        let content_bytes = content.as_bytes();

        if offset >= content_bytes.len() {
            return Ok(0); // EOF
        }

        let to_copy = core::cmp::min(buffer.len(), content_bytes.len() - offset);
        buffer[..to_copy].copy_from_slice(&content_bytes[offset..offset + to_copy]);
        Ok(to_copy)
    }

    fn write(&self, _offset: usize, _buffer: &[u8]) -> Result<usize, VfsError> {
        // Proc files are read-only
        Err(VfsError::PermissionDenied)
    }

    fn file_type(&self) -> FileType {
        FileType::Regular // Proc files appear as regular files
    }

    fn size(&self) -> usize {
        let proc_file = self.lock();
        proc_file.generate_content().len()
    }

    fn lookup(&self, _name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        Err(VfsError::NotADirectory)
    }

    fn create(&self, _name: &str, _file_type: FileType) -> Result<Arc<dyn Inode>, VfsError> {
        Err(VfsError::NotADirectory)
    }

    fn list(&self) -> Result<Vec<String>, VfsError> {
        Err(VfsError::NotADirectory)
    }

    fn truncate(&self, _size: usize) -> Result<(), VfsError> {
        Err(VfsError::PermissionDenied)
    }

    fn remove(&self, _name: &str) -> Result<(), VfsError> {
        Err(VfsError::NotADirectory)
    }
}

/// ProcFS - Process filesystem
pub struct ProcFs {
    files: Mutex<Vec<(String, Arc<dyn Inode>)>>,
}

impl ProcFs {
    pub fn new() -> Self {
        let procfs = Self {
            files: Mutex::new(Vec::new()),
        };

        // Create standard proc files
        procfs.add_file("meminfo", ProcFile::new(ProcFileType::MemInfo, "meminfo"));

        procfs
    }

    fn add_file(&self, name: &str, file: Arc<dyn Inode>) {
        let mut files = self.files.lock();
        files.push((String::from(name), file));
    }

    pub fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let files = self.files.lock();
        for (file_name, file_inode) in files.iter() {
            if file_name == name {
                return Ok(file_inode.clone());
            }
        }
        Err(VfsError::NotFound)
    }

    pub fn list(&self) -> Result<Vec<String>, VfsError> {
        let files = self.files.lock();
        Ok(files.iter().map(|(name, _)| name.clone()).collect())
    }
}
