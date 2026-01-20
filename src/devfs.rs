//! Device Filesystem (devfs)
//!
//! Provides special device files like /dev/null, /dev/zero, /dev/tty

use crate::vfs::{Inode, FileType, VfsError};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;

/// Device types supported by devfs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Null,    // Discards all writes, returns EOF on reads
    Zero,    // Provides infinite stream of zeros
    Console, // Console/tty device
}

/// A device node in devfs
pub struct DeviceNode {
    dev_type: DeviceType,
    #[allow(dead_code)]
    name: String,
}

impl DeviceNode {
    pub fn new(dev_type: DeviceType, name: &str) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            dev_type,
            name: String::from(name),
        }))
    }
}

impl Inode for Mutex<DeviceNode> {
    fn read(&self, _offset: usize, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let dev = self.lock();
        match dev.dev_type {
            DeviceType::Null => {
                // /dev/null returns EOF (0 bytes)
                Ok(0)
            }
            DeviceType::Zero => {
                // /dev/zero fills buffer with zeros
                for byte in buffer.iter_mut() {
                    *byte = 0;
                }
                Ok(buffer.len())
            }
            DeviceType::Console => {
                // For now, console read is not implemented
                // TODO: Implement keyboard buffer reading
                Err(VfsError::NotImplemented)
            }
        }
    }

    fn write(&self, _offset: usize, buffer: &[u8]) -> Result<usize, VfsError> {
        let dev = self.lock();
        match dev.dev_type {
            DeviceType::Null => {
                // /dev/null discards all writes
                Ok(buffer.len())
            }
            DeviceType::Zero => {
                // /dev/zero is read-only
                Err(VfsError::PermissionDenied)
            }
            DeviceType::Console => {
                // Write to console via VGA/serial
                use crate::{print, serial_print};
                if let Ok(s) = core::str::from_utf8(buffer) {
                    print!("{}", s);
                    serial_print!("{}", s);
                }
                Ok(buffer.len())
            }
        }
    }

    fn file_type(&self) -> FileType {
        FileType::Device
    }

    fn size(&self) -> usize {
        0 // Device nodes don't have a meaningful size
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
        Err(VfsError::InvalidOperation)
    }

    fn remove(&self, _name: &str) -> Result<(), VfsError> {
        Err(VfsError::NotADirectory)
    }
}

/// DevFS - Device filesystem
pub struct DevFs {
    devices: Mutex<Vec<(String, Arc<dyn Inode>)>>,
}

impl DevFs {
    pub fn new() -> Self {
        let devfs = Self {
            devices: Mutex::new(Vec::new()),
        };

        // Create standard device nodes
        devfs.add_device("null", DeviceNode::new(DeviceType::Null, "null"));
        devfs.add_device("zero", DeviceNode::new(DeviceType::Zero, "zero"));
        devfs.add_device("console", DeviceNode::new(DeviceType::Console, "console"));
        devfs.add_device("tty", DeviceNode::new(DeviceType::Console, "tty")); // tty is an alias for console

        devfs
    }

    fn add_device(&self, name: &str, device: Arc<dyn Inode>) {
        let mut devices = self.devices.lock();
        devices.push((String::from(name), device));
    }

    pub fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let devices = self.devices.lock();
        for (dev_name, dev_inode) in devices.iter() {
            if dev_name == name {
                return Ok(dev_inode.clone());
            }
        }
        Err(VfsError::NotFound)
    }

    pub fn list(&self) -> Result<Vec<String>, VfsError> {
        let devices = self.devices.lock();
        Ok(devices.iter().map(|(name, _)| name.clone()).collect())
    }
}
