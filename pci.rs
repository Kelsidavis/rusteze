// src/pci.rs

use x86_64::instructions::port::{Port, PortReadOnly};
use core::fmt;

/// Represents a PCI device function.
#[derive(Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
}

impl fmt::Display for PciDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PCI {}:{}.{}", self.bus, self.slot, self.func)
    }
}

/// PCI Configuration Space Access
const CONFIG_ADDRESS_PORT: u16 = 0xCF8;
const CONFIG_DATA_PORT: u16 = 0xCFC;

// Helper function to generate the configuration address for a given device and register offset
fn make_config_address(bus: u8, slot: u8, func: u8, reg_offset: u32) -> u32 {
    // The format is:
    // Bit [31] = 0 (reserved)
    // Bits [30..24] = bus number
    // Bits [23..16] = slot number  
    // Bits [15..12] = function number
    // Bits [11..0] = register offset
    
    let mut address: u32 = 0;
    
    // Set the header type bit (bit 31)
    address |= 1 << 31;
    
    // Add bus, slot and func numbers to bits 24-12
    address |= ((bus as u32) & 0xFF) << 24;
    address |= ((slot as u32) & 0x1F) << 16;
    address |= ((func as u32) & 0x7) << 12;
    
    // Add register offset
    address |= (reg_offset & 0xFFF);
    
    address
}

/// Read a 32-bit value from PCI configuration space.
fn read_config_dword(bus: u8, slot: u8, func: u8, reg_offset: u32) -> Option<u32> {
    let mut config_address = Port::new(CONFIG_ADDRESS_PORT);
    
    // Write the address to the port
    unsafe { 
        config_address.write(make_config_address(bus, slot, func, reg_offset));
        
        // Read from data port and return result
        Some(PortReadOnly::<u32>::new(CONFIG_DATA_PORT).read())
    }
}

/// Check if a PCI device exists by reading its vendor ID.
fn is_device_present(bus: u8, slot: u8, func: u8) -> bool {
    // Read the Vendor ID (offset 0x0)
    let vid = read_config_dword(bus, slot, func, 0);
    
    match vid {
        Some(vid) => vid != 0xFFFF && vid != 0,
        None => false
    }
}

/// Enumerate all PCI devices on a given bus.
pub fn enumerate_bus(bus: u8) -> Vec<PciDevice> {
    let mut devices = vec![];
    
    // Scan through slots (0-31)
    for slot in 0..=31u8 {
        // Check if there's at least one function present
        if is_device_present(bus, slot, 0) {
            // Device exists - check all functions (up to 8 per device)
            let mut func = 0;
            
            while func < 8 && is_device_present(bus, slot, func) {
                devices.push(PciDevice { bus, slot, func });
                
                if !is_multi_function_device(bus, slot, func) {
                    break; // Only one function for this device
                }
                
                func += 1;
            }
        }
    }
    
    devices
}

/// Check if a PCI device supports multiple functions.
fn is_multi_function_device(bus: u8, slot: u8, func: u8) -> bool {
    let header_type = read_config_dword(bus, slot, func, 0x0C).unwrap_or(0);
    
    // Bit 7 of the Header Type register indicates multi-function capability
    (header_type & 0x80) != 0
}

/// Get a PCI device's vendor ID.
pub fn get_vendor_id(device: &PciDevice) -> Option<u16> {
    let vid = read_config_dword(device.bus, device.slot, device.func, 0);
    
    match vid {
        Some(v) => Some((v >> 16) as u16),
        None => None
    }
}

/// Get a PCI device's device ID.
pub fn get_device_id(device: &PciDevice) -> Option<u16> {
    let did = read_config_dword(device.bus, device.slot, device.func, 0);
    
    match did {
        Some(v) => Some((v >> 0) as u16),
        None => None
    }
}

/// Get a PCI device's class code.
pub fn get_class_code(device: &PciDevice) -> Option<u8> {
    let cc = read_config_dword(device.bus, device.slot, device.func, 0x0A);
    
    match cc {
        Some(v) => Some((v >> 16) as u8),
        None => None
    }
}

/// Get a PCI device's subclass code.
pub fn get_subclass_code(device: &PciDevice) -> Option<u8> {
    let sc = read_config_dword(device.bus, device.slot, device.func, 0x0A);
    
    match sc {
        Some(v) => Some((v >> 8) as u8),
        None => None
    }
}

/// Get a PCI device's programming interface.
pub fn get_programming_interface(device: &PciDevice) -> Option<u8> {
    let pi = read_config_dword(device.bus, device.slot, device.func, 0x0A);
    
    match pi {
        Some(v) => Some((v >> 16) as u8),
        None => None
    }
}

/// Get a PCI device's revision ID.
pub fn get_revision_id(device: &PciDevice) -> Option<u8> {
    let rid = read_config_dword(device.bus, device.slot, device.func, 0x0A);
    
    match rid {
        Some(v) => Some((v >> 24) as u8),
        None => None
    }
}

/// Get a PCI device's base address registers (BARs).
pub fn get_bar(device: &PciDevice, bar_index: usize) -> Option<u64> {
    if bar_index > 5 { return None; } // Only support up to BAR5
    
    let offset = 0x10 + (bar_index * 4);
    
    read_config_dword(device.bus, device.slot, device.func, offset as u32)
        .map(|val| val & !0xF) // Clear the bottom 4 bits
}

/// Get a PCI device's command register.
pub fn get_command_register(device: &PciDevice) -> Option<u16> {
    let cmd = read_config_dword(device.bus, device.slot, device.func, 0x04);
    
    match cmd {
        Some(v) => Some((v >> 0) as u16),
        None => None
    }
}

/// Get a PCI device's status register.
pub fn get_status_register(device: &PciDevice) -> Option<u16> {
    let stat = read_config_dword(device.bus, device.slot, device.func, 0x04);
    
    match stat {
        Some(v) => Some((v >> 16) as u16),
        None => None
    }
}

/// Initialize PCI enumeration.
pub fn init_pci() -> Vec<PciDevice> {
    let mut devices = vec![];
    
    // Scan all buses (0-255)
    for bus in 0..=255u8 {
        if !is_device_present(bus, 0x1F, 0) { 
            continue; // No PCI bridge on this bus
        }
        
        let mut found_devices = enumerate_bus(bus);
        devices.append(&mut found_devices);
    }
    
    println!("PCI enumeration complete: {} device(s)", devices.len());
    
    for dev in &devices {
        if let Some(vid) = get_vendor_id(dev) {
            if let Some(did) = get_device_id(dev) {
                // Print basic info about each discovered PCI device
                match (get_class_code(dev), get_subclass_code(dev)) {
                    (Some(class), Some(subclass)) => println!(
                        "PCI Device: {} - Vendor ID 0x{:X}, Device ID 0x{:X} ({:#06X}:{:#04X})",
                        dev, vid, did,
                        class << 8 | subclass, 
                        get_programming_interface(dev).unwrap_or(0)
                    ),
                }
            }
        } else {
            println!("PCI Device: {} - Unknown vendor/device ID", dev);
        }
    }

    devices
}
