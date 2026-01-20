//! TmpFS - In-memory filesystem
//!
//! A simple RAM-based filesystem for temporary files and directories

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use spin::Mutex;

use crate::vfs::{Inode, FileType, VfsError};

/// TmpFS inode - can be a file or directory
pub struct TmpFsInode {
    inner: Mutex<TmpFsInodeInner>,
}

struct TmpFsInodeInner {
    file_type: FileType,
    data: Vec<u8>,
    children: BTreeMap<String, Arc<TmpFsInode>>,
}

impl TmpFsInode {
    /// Create a new file inode
    pub fn new_file() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(TmpFsInodeInner {
                file_type: FileType::Regular,
                data: Vec::new(),
                children: BTreeMap::new(),
            }),
        })
    }

    /// Create a new directory inode
    pub fn new_directory() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(TmpFsInodeInner {
                file_type: FileType::Directory,
                data: Vec::new(),
                children: BTreeMap::new(),
            }),
        })
    }
}

impl Inode for TmpFsInode {
    fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let inner = self.inner.lock();

        if inner.file_type != FileType::Regular {
            return Err(VfsError::IsADirectory);
        }

        if offset >= inner.data.len() {
            return Ok(0);
        }

        let available = inner.data.len() - offset;
        let to_read = buffer.len().min(available);

        buffer[..to_read].copy_from_slice(&inner.data[offset..offset + to_read]);
        Ok(to_read)
    }

    fn write(&self, offset: usize, buffer: &[u8]) -> Result<usize, VfsError> {
        let mut inner = self.inner.lock();

        if inner.file_type != FileType::Regular {
            return Err(VfsError::IsADirectory);
        }

        // Extend the data vector if necessary
        let required_size = offset + buffer.len();
        if required_size > inner.data.len() {
            inner.data.resize(required_size, 0);
        }

        // Write the data
        inner.data[offset..offset + buffer.len()].copy_from_slice(buffer);
        Ok(buffer.len())
    }

    fn file_type(&self) -> FileType {
        self.inner.lock().file_type
    }

    fn size(&self) -> usize {
        self.inner.lock().data.len()
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let inner = self.inner.lock();

        if inner.file_type != FileType::Directory {
            return Err(VfsError::NotADirectory);
        }

        inner
            .children
            .get(name)
            .cloned()
            .map(|inode| inode as Arc<dyn Inode>)
            .ok_or(VfsError::NotFound)
    }

    fn create(&self, name: &str, file_type: FileType) -> Result<Arc<dyn Inode>, VfsError> {
        let mut inner = self.inner.lock();

        if inner.file_type != FileType::Directory {
            return Err(VfsError::NotADirectory);
        }

        if inner.children.contains_key(name) {
            return Err(VfsError::AlreadyExists);
        }

        let new_inode = match file_type {
            FileType::Regular => TmpFsInode::new_file(),
            FileType::Directory => TmpFsInode::new_directory(),
            _ => return Err(VfsError::NotImplemented),
        };

        inner.children.insert(name.to_string(), new_inode.clone());
        Ok(new_inode)
    }

    fn list(&self) -> Result<Vec<String>, VfsError> {
        let inner = self.inner.lock();

        if inner.file_type != FileType::Directory {
            return Err(VfsError::NotADirectory);
        }

        Ok(inner.children.keys().cloned().collect())
    }

    fn truncate(&self, size: usize) -> Result<(), VfsError> {
        let mut inner = self.inner.lock();

        if inner.file_type != FileType::Regular {
            return Err(VfsError::IsADirectory);
        }

        inner.data.resize(size, 0);
        Ok(())
    }
}

/// TmpFS filesystem
pub struct TmpFs {
    root: Arc<TmpFsInode>,
}

impl TmpFs {
    /// Create a new TmpFS filesystem
    pub fn new() -> Self {
        Self {
            root: TmpFsInode::new_directory(),
        }
    }

    /// Get the root inode
    pub fn root(&self) -> Arc<dyn Inode> {
        self.root.clone()
    }

    /// Resolve a path to an inode
    pub fn resolve_path(&self, path: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Ok(self.root.clone());
        }

        let components: Vec<&str> = path.split('/').collect();
        let mut current: Arc<dyn Inode> = self.root.clone();

        for component in components {
            if component.is_empty() || component == "." {
                continue;
            }

            current = current.lookup(component)?;
        }

        Ok(current)
    }

    /// Create a file at the given path
    pub fn create_file(&self, path: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Err(VfsError::InvalidArgument);
        }

        let components: Vec<&str> = path.split('/').collect();
        let (parent_components, filename) = components.split_at(components.len() - 1);

        let filename = filename[0];

        // Navigate to parent directory
        let mut current: Arc<dyn Inode> = self.root.clone();
        for component in parent_components {
            if component.is_empty() || *component == "." {
                continue;
            }
            current = current.lookup(component)?;
        }

        // Create the file
        current.create(filename, FileType::Regular)
    }

    /// Create a directory at the given path
    pub fn create_directory(&self, path: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Err(VfsError::InvalidArgument);
        }

        let components: Vec<&str> = path.split('/').collect();
        let (parent_components, dirname) = components.split_at(components.len() - 1);

        let dirname = dirname[0];

        // Navigate to parent directory
        let mut current: Arc<dyn Inode> = self.root.clone();
        for component in parent_components {
            if component.is_empty() || *component == "." {
                continue;
            }
            current = current.lookup(component)?;
        }

        // Create the directory
        current.create(dirname, FileType::Directory)
    }
}

impl Default for TmpFs {
    fn default() -> Self {
        Self::new()
    }
}

use lazy_static::lazy_static;

lazy_static! {
    /// Global TmpFS instance
    pub static ref TMPFS: Mutex<TmpFs> = Mutex::new(TmpFs::new());
}
