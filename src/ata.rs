// src/ata.rs

use crate::println;
use x86_64::{instructions::port::Port, structures::paging::PhysFrame};
use core::ptr;

/// Primary IDE channel base port addresses (for master and slave)
const PRIMARY_DATA_PORT: u16 = 0x1F0;
const PRIMARY_ERROR_PORT: u16 = 0x1F1; // Read-only
const PRIMARY_FEATURES_PORT: u16 = 0x1F2; // Write-only, for error recovery features (not used)
const PRIMARY_SECTOR_COUNT_PORT: u16 = 0x1F3;
const PRIMARY_LBA_LOW_PORT: u16 = 0x1F4;
const PRIMARY_LBA_MID_PORT: u16 = 0x1F5;
const PRIMARY_LBA_HIGH_PORT: u16 = 0x1F6; // Read/write, contains LBA bits and device select
const PRIMARY_COMMAND_PORT: u16 = 0x1F7;

/// Secondary IDE channel base port addresses (for master and slave)
const SECONDARY_DATA_PORT: u16 = 0x170;
const SECONDARY_ERROR_PORT: u16 = 0x171; // Read-only
const SECONDARY_FEATURES_PORT: u16 = 0x172; // Write-only, for error recovery features (not used)
const SECONDARY_SECTOR_COUNT_PORT: u16 = 0x173;
const SECONDARY_LBA_LOW_PORT: u16 = 0x174;
const SECONDARY_LBA_MID_PORT: u16 = 0x175;
const SECONDARY_LBA_HIGH_PORT: u16 = 0x176; // Read/write, contains LBA bits and device select
const SECONDARY_COMMAND_PORT: u16 = 0x177;

/// ATA command codes (from the specification)
#[allow(dead_code)]
enum AtaCommand {
    IdentifyDevice = 0xEC,
    ReadSectorsWithRetry = 0x20,
    WriteSectorsWithRetry = 0x30,
}

// Status register bits
const STATUS_BSY: u8 = 1 << 7; // Busy bit (set when device is busy)
const STATUS_DRDY: u8 = 1 << 6; // Device ready bit
const STATUS_DF: u8 = 1 << 5; // Drive fault error flag
const STATUS_DSC: u8 = 1 << 4; // Seek complete - not used in PIO mode
const STATUS_DRQ: u8 = 1 << 3; // Data request (device ready to transfer data)
const STATUS_CORR: u8 = 1 << 2; // Corrected error bit
const STATUS_IDX: u8 = 1 << 1; // Index - not used in PIO mode
const STATUS_ERR: u8 = 1 << 0; // Error flag

// Device select bits (in LBA_HIGH_PORT)
const DEVICE_MASTER: u8 = 0x00;
const DEVICE_SLAVE: u8 = 0x10;

/// ATA/IDE device structure to represent a disk
pub struct AtaDisk {
    pub channel: Channel,
    pub is_slave: bool,
}

impl AtaDisk {
    /// Create a new AtaDisk instance for the specified channel and slave/master status.
    pub fn new(channel: Channel, is_slave: bool) -> Self {
        Self { channel, is_slave }
    }

    /// Initialize the disk by sending an IDENTIFY command to get device information
    pub fn init(&self) -> Result<AtaDeviceIdentifyInfo, &'static str> {
        // Wait for any previous operation to complete and ensure we're not busy
        self.wait_for_ready()?;

        let mut port = Port::new(match self.channel {
            Channel::Primary => PRIMARY_COMMAND_PORT,
            Channel::Secondary => SECONDARY_COMMAND_PORT,
        });

        // Send the IDENTIFY command (0xEC)
        unsafe { 
            port.write(AtaCommand::IdentifyDevice as u8);
        }

        // Wait for DRQ bit to be set, indicating data is ready
        self.wait_for_drq()?;

        let mut info = AtaDeviceIdentifyInfo {
            device_type: 0,
            sectors_per_track: 0,
            heads: 0,
            total_sectors: 0,
            model_number: [0; 41],
            serial_number: [0; 21],
            firmware_revision: [0; 9],
        };

        // Read the identify data (512 bytes = 256 words)
        let mut buffer_ptr = info as *mut AtaDeviceIdentifyInfo;
        
        for i in 0..=255 {
            unsafe { 
                ptr::write_volatile(
                    &mut (*buffer_ptr).model_number[i] as *mut u8,
                    self.read_word()?,
                );
                
                // Skip the word we just read
                let _ = self.read_word()?;
            }
        }

        Ok(info)
    }

    /// Wait for device to be ready (not busy and DRDY set).
    fn wait_for_ready(&self) -> Result<(), &'static str> {
        loop {
            match unsafe { 
                Port::new(match self.channel {
                    Channel::Primary => PRIMARY_COMMAND_PORT,
                    Channel::Secondary => SECONDARY_COMMAND_PORT,
                }).read()
            } & (STATUS_BSY | STATUS_DRDY)
            {
                0x40 => break, // DRDY set and not busy
                _ => continue,
            }
        }

        Ok(())
    }

    /// Wait for the device to be ready to transfer data.
    fn wait_for_drq(&self) -> Result<(), &'static str> {
        loop {
            let status = unsafe { 
                Port::new(match self.channel {
                    Channel::Primary => PRIMARY_COMMAND_PORT,
                    Channel::Secondary => SECONDARY_COMMAND_PORT,
                }).read()
            };

            if (status & STATUS_DRQ) != 0 && (status & STATUS_BSY) == 0 {
                break;
            }
        }

        Ok(())
    }

    /// Read a word from the data port.
    fn read_word(&self) -> Result<u16, &'static str> {
        let mut port = Port::new(match self.channel {
            Channel::Primary => PRIMARY_DATA_PORT,
            Channel::Secondary => SECONDARY_DATA_PORT,
        });

        unsafe { 
            Ok(port.read())
        }
    }

    /// Read a sector from the disk using LBA addressing.
    pub fn read_sector(&self, lba: u64) -> Result<[u8; 512], &'static str> {
        // Wait for device to be ready
        self.wait_for_ready()?;

        let mut port = Port::new(match self.channel {
            Channel::Primary => PRIMARY_COMMAND_PORT,
            Channel::Secondary => SECONDARY_COMMAND_PORT,
        });

        unsafe { 
            // Set up the LBA address (28 bits)
            port.write(0x40 | if self.is_slave { 1 } else { 0 }); // Device select + LBA mode
            let lba_bytes = [
                ((lba >> 24) & 0xFF) as u8,
                ((lba >> 16) & 0xFF) as u8,
                ((lba >> 8) & 0xFF) as u8,
                (lba & 0xFF) as u8
            ];
            
            // Write LBA bits to the appropriate ports in order
            Port::new(match self.channel {
                Channel::Primary => PRIMARY_LBA_LOW_PORT,
                Channel::Secondary => SECONDARY_LBA_LOW_PORT,
            }).write(lba_bytes[3]);
                
            Port::new(match self.channel {
                Channel::Primary => PRIMARY_LBA_MID_PORT,
                Channel::Secondary => SECONDARY_LBA_MID_PORT,
            }).write(lba_bytes[2]);

            Port::new(match self.channel {
                Channel::Primary => PRIMARY_LBA_HIGH_PORT,
                Channel::Secondary => SECONDARY_LBA_HIGH_PORT,
            }).write(lba_bytes[1] | (lba_bytes[0] & 0x0F) << 4);

            // Set sector count to 1
            Port::new(match self.channel {
                Channel::Primary => PRIMARY_SECTOR_COUNT_PORT,
                Channel::Secondary => SECONDARY_SECTOR_COUNT_PORT,
            }).write(1);
        }

        // Send the READ SECTORS command (0x20)
        unsafe { 
            port.write(AtaCommand::ReadSectorsWithRetry as u8);
        }

        self.wait_for_drq()?;

        let mut buffer = [0u8; 512];
        
        for i in 0..=255 {
            // Read a word from the data port
            unsafe { 
                ptr::write_volatile(
                    &mut buffer[i * 2] as *mut u8,
                    Port::new(match self.channel {
                        Channel::Primary => PRIMARY_DATA_PORT,
                        Channel::Secondary => SECONDARY_DATA_PORT,
                    }).read() as u8,
                );
                
                // Skip the word we just read
                let _ = Port::new(match self.channel {
                    Channel::Primary => PRIMARY_DATA_PORT,
                    Channel::Secondary => SECONDARY_DATA_PORT,
                }).read();
            }
        }

        Ok(buffer)
    }

    /// Write a sector to the disk using LBA addressing.
    pub fn write_sector(&self, lba: u64, data: &[u8; 512]) -> Result<(), &'static str> {
        // Wait for device to be ready
        self.wait_for_ready()?;

        let mut port = Port::new(match self.channel {
            Channel::Primary => PRIMARY_COMMAND_PORT,
            Channel::Secondary => SECONDARY_COMMAND_PORT,
        });

        unsafe { 
            // Set up the LBA address (28 bits)
            port.write(0x40 | if self.is_slave { 1 } else { 0 }); // Device select + LBA mode
            let lba_bytes = [
                ((lba >> 24) & 0xFF) as u8,
                ((lba >> 16) & 0xFF) as u8,
                ((lba >> 8) & 0xFF) as u8,
                (lba & 0xFF) as u8
            ];
            
            // Write LBA bits to the appropriate ports in order
            Port::new(match self.channel {
                Channel::Primary => PRIMARY_LBA_LOW_PORT,
                Channel::Secondary => SECONDARY_LBA_LOW_PORT,
            }).write(lba_bytes[3]);
                
            Port::new(match self.channel {
                Channel::Primary => PRIMARY_LBA_MID_PORT,
                Channel::Secondary => SECONDARY_LBA_MID_PORT,
            }).write(lba_bytes[2]);

            Port::new(match self.channel {
                Channel::Primary => PRIMARY_LBA_HIGH_PORT,
                Channel::Secondary => SECONDARY_LBA_HIGH_PORT,
            }).write(lba_bytes[1] | (lba_bytes[0] & 0x0F) << 4);

            // Set sector count to 1
            Port::new(match self.channel {
                Channel::Primary => PRIMARY_SECTOR_COUNT_PORT,
                Channel::Secondary => SECONDARY_SECTOR_COUNT_PORT,
            }).write(1);
        }

        // Send the WRITE SECTORS command (0x30)
        unsafe { 
            port.write(AtaCommand::WriteSectorsWithRetry as u8);
        }

        self.wait_for_drq()?;

        for i in 0..=255 {
            let word = ((data[i * 2] as u16) << 8) | (data[(i*2)+1] as u16);

            unsafe { 
                Port::new(match self.channel {
                    Channel::Primary => PRIMARY_DATA_PORT,
                    Channel::Secondary => SECONDARY_DATA_PORT,
                }).write(word);
                
                // Wait for the device to be ready
                let mut status = 0;
                loop {
                    status = Port::new(match self.channel {
                        Channel::Primary => PRIMARY_COMMAND_PORT,
                        Channel::Secondary => SECONDARY_COMMAND_PORT,
                    }).read();
                    
                    if (status & STATUS_DRQ) == 0 && (status & STATUS_BSY) == 0 { break; }
                }

            }
        }

        // Wait for the operation to complete
        self.wait_for_ready()?;

        Ok(())
    }
}

/// Channel enumeration for primary and secondary IDE channels.
#[derive(Debug, Clone)]
pub enum Channel {
    Primary,
    Secondary,
}

impl From<Channel> for u8 {
    fn from(channel: Channel) -> Self {
        match channel {
            Channel::Primary => 0x1F0,
            Channel::Secondary => 0x170,
        }
    }
}

/// Structure to hold the identify data returned by an ATA device.
#[repr(C)]
pub struct AtaDeviceIdentifyInfo {
    pub device_type: u8, // Device type (bit 2 = removable)
    pub sectors_per_track: u16,
    pub heads: u16,
    pub total_sectors: u32,

    /// Model number string - null-terminated
    pub model_number: [u8; 41],

    /// Serial number string - null-terminated  
    pub serial_number: [u8; 21],

    /// Firmware revision string - null-terminated
    pub firmware_revision: [u8; 9],
}

impl AtaDeviceIdentifyInfo {
    // Helper method to get the model name as a &str (if valid)
    #[allow(dead_code)]
    fn model_name(&self) -> Option<&str> {
        let bytes = self.model_number.as_slice();
        
        if !bytes.is_empty() && bytes[0] != 0x20 { // Check for non-space first byte
            return unsafe { 
                core::slice::from_raw_parts(bytes.as_ptr(), bytes.len())
                    .split(|&b| b == 0)
                    .next()
                    .map(|s| {
                        let s = &s[1..]; // Skip the leading space (if any) - some devices have it
                        if !s.is_empty() && *s.first().unwrap_or(&0x20) != 0x20 { 
                            core::str::from_utf8_unchecked(s)
                        } else {
                            ""
                        }
                    })
            };
        }

        None
    }

    // Helper method to get the serial number as a &str (if valid)
    #[allow(dead_code)]
    fn serial_number_str(&self) -> Option<&str> {
        let bytes = self.serial_number.as_slice();
        
        if !bytes.is_empty() && bytes[0] != 0x20 { // Check for non-space first byte
            return unsafe { 
                core::slice::from_raw_parts(bytes.as_ptr(), bytes.len())
                    .split(|&b| b == 0)
                    .next()
                    .map(|s| {
                        let s = &s[1..]; // Skip the leading space (if any) - some devices have it
                        if !s.is_empty() && *s.first().unwrap_or(&0x20) != 0x20 { 
                            core::str::from_utf8_unchecked(s)
                        } else {
                            ""
                        }
                    })
            };
        }

        None
    }

    // Helper method to get the firmware revision as a &str (if valid)
    #[allow(dead_code)]
    fn firmware_revision_str(&self) -> Option<&str> {
        let bytes = self.firmware_revision.as_slice();
        
        if !bytes.is_empty() && bytes[0] != 0x20 { // Check for non-space first byte
            return unsafe { 
                core::slice::from_raw_parts(bytes.as_ptr(), bytes.len())
                    .split(|&b| b == 0)
                    .next()
                    .map(|s| {
                        let s = &s[1..]; // Skip the leading space (if any) - some devices have it
                        if !s.is_empty() && *s.first().unwrap_or(&0x20) != 0x20 { 
                            core::str::from_utf8_unchecked(s)
                        } else {
                            ""
                        }
                    })
            };
        }

        None
    }
}

/// Initialize the ATA/IDE disk driver.
pub fn init_ata() -> Vec<AtaDisk> {
    let mut disks = vec![];

    // Check primary channel (master and slave devices)
    for is_slave in [false, true] {
        let master_disk = AtaDisk::new(Channel::Primary, is_slave);
        
        match master_disk.init() {
            Ok(info) => {
                println!(
                    "ATA Disk: {} - Model '{}', Serial '{}' ({})",
                    if is_slave { "Slave" } else { "Master" },
                    info.model_name().unwrap_or("Unknown"),
                    info.serial_number_str().unwrap_or("Unknown"),
                    match info.device_type & 0x4 {
                        0 => "Fixed Disk", 
                        _ => "Removable"
                    }
                );
                
                disks.push(master_disk);
            },
            Err(_) => continue,
        };
    }

    // Check secondary channel (master and slave devices)
    for is_slave in [false, true] {
        let master_disk = AtaDisk::new(Channel::Secondary, is_slave);

        match master_disk.init() {
            Ok(info) => {
                println!(
                    "ATA Disk: {} - Model '{}', Serial '{}' ({})",
                    if is_slave { "Slave" } else { "Master" },
                    info.model_name().unwrap_or("Unknown"),
                    info.serial_number_str().unwrap_or("Unknown"),
                    match info.device_type & 0x4 {
                        0 => "Fixed Disk", 
                        _ => "Removable"
                    }
                );
                
                disks.push(master_disk);
            },
            Err(_) => continue,
        };
    }

    println!("ATA/IDE driver initialized: {} disk(s) found.", disks.len());
    
    // Test reading a sector from the first available drive
    if let Some(disk) = disks.first() {
        match disk.read_sector(0x12345678) {
            Ok(data) => println!("Successfully read sector 0x{:X} (first 16 bytes: {:?})", 
                0x12345678, &data[..16]),
            Err(e) => println!("Failed to read from disk: {}", e),
        }
    }

    disks
}
