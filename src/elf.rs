//! ELF binary loader for static binaries
//!
//! Provides functionality to parse and load ELF executables into memory
//!
//! Note: This module is infrastructure for future userspace program loading.
//! It will be used once the Ring 0 -> Ring 3 transition is implemented.

#![allow(dead_code)]

use alloc::vec::Vec;
use core::mem;

/// ELF magic number
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class (32-bit vs 64-bit)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    Elf32 = 1,
    Elf64 = 2,
}

/// ELF data encoding (endianness)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfData {
    LittleEndian = 1,
    BigEndian = 2,
}

/// ELF file type
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfType {
    None = 0,
    Relocatable = 1,
    Executable = 2,
    Shared = 3,
    Core = 4,
}

/// Program header type
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhType {
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interp = 3,
    Note = 4,
}

/// ELF64 header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// ELF64 program header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// Parsed ELF binary
pub struct ElfBinary<'a> {
    data: &'a [u8],
    header: Elf64Header,
}

impl<'a> ElfBinary<'a> {
    /// Parse an ELF binary from a byte slice
    pub fn parse(data: &'a [u8]) -> Result<Self, &'static str> {
        if data.len() < mem::size_of::<Elf64Header>() {
            return Err("Data too small for ELF header");
        }

        // Safety: We've checked the size
        let header: Elf64Header = unsafe {
            core::ptr::read(data.as_ptr() as *const Elf64Header)
        };

        // Check magic number
        if header.e_ident[0..4] != ELF_MAGIC {
            return Err("Invalid ELF magic number");
        }

        // Check class (must be 64-bit)
        if header.e_ident[4] != ElfClass::Elf64 as u8 {
            return Err("Only 64-bit ELF supported");
        }

        // Check data encoding (must be little endian)
        if header.e_ident[5] != ElfData::LittleEndian as u8 {
            return Err("Only little-endian ELF supported");
        }

        // Check version
        if header.e_ident[6] != 1 {
            return Err("Invalid ELF version");
        }

        // Check type (must be executable)
        if header.e_type != ElfType::Executable as u16 {
            return Err("Only executable ELF supported");
        }

        // Check machine (must be x86-64)
        if header.e_machine != 0x3E {
            return Err("Only x86-64 ELF supported");
        }

        Ok(Self { data, header })
    }

    /// Get the entry point address
    pub fn entry_point(&self) -> u64 {
        self.header.e_entry
    }

    /// Get program headers
    pub fn program_headers(&self) -> Result<Vec<Elf64ProgramHeader>, &'static str> {
        let phoff = self.header.e_phoff as usize;
        let phentsize = self.header.e_phentsize as usize;
        let phnum = self.header.e_phnum as usize;

        if phoff + phentsize * phnum > self.data.len() {
            return Err("Program headers out of bounds");
        }

        let mut headers = Vec::new();
        for i in 0..phnum {
            let offset = phoff + i * phentsize;
            let ph: Elf64ProgramHeader = unsafe {
                core::ptr::read(self.data[offset..].as_ptr() as *const Elf64ProgramHeader)
            };
            headers.push(ph);
        }

        Ok(headers)
    }

    /// Get the data for a program segment
    pub fn segment_data(&self, ph: &Elf64ProgramHeader) -> Result<&[u8], &'static str> {
        let offset = ph.p_offset as usize;
        let filesz = ph.p_filesz as usize;

        if offset + filesz > self.data.len() {
            return Err("Segment data out of bounds");
        }

        Ok(&self.data[offset..offset + filesz])
    }
}

/// Load an ELF binary into memory and return the entry point
///
/// This function:
/// 1. Parses the ELF header
/// 2. Loads all PT_LOAD segments into memory
/// 3. Sets up the BSS section (zero-initialized data)
/// 4. Returns the entry point address
///
/// Note: This is a simplified loader for static binaries only.
/// Dynamic linking is not supported.
pub fn load_elf(data: &[u8]) -> Result<u64, &'static str> {
    let elf = ElfBinary::parse(data)?;
    let program_headers = elf.program_headers()?;

    // Load all PT_LOAD segments
    for ph in program_headers.iter() {
        if ph.p_type == PhType::Load as u32 {
            // Get the segment data
            let segment_data = elf.segment_data(ph)?;

            // For now, we just validate that we can access the data
            // In a real implementation, we would:
            // 1. Allocate pages for the segment
            // 2. Map the pages into the process address space
            // 3. Copy the segment data to the mapped pages
            // 4. Zero-fill the BSS section if memsz > filesz

            if segment_data.len() != ph.p_filesz as usize {
                return Err("Segment data size mismatch");
            }

            // TODO: Actually load the segment into memory
            // This requires integration with the paging system
        }
    }

    Ok(elf.entry_point())
}

/// Create a simple embedded test binary
///
/// This returns a minimal static ELF binary that can be used for testing
/// the loader without needing external files.
#[allow(dead_code)]
pub fn create_test_binary() -> Vec<u8> {
    // This would contain a minimal x86-64 ELF binary
    // For now, just return an empty vector as a placeholder
    Vec::new()
}
