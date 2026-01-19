// src/shell.rs

use crate::vfs::{Vfs, Inode};
use core::fmt;

/// A simple shell environment.
pub struct Shell {
    /// The current working directory path (as a string).
    pub cwd: String,
}

impl fmt::Write for Shell {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // This is just to satisfy the trait requirement
        Ok(())
    }
}

// Initialize a new shell with default settings and current working directory (cwd)
impl Default for Shell {
    fn default() -> Self {
        let cwd = String::from("/");
        
        Self { cwd }
    }
}

/// Execute the `cat` command to display file contents using VFS
pub fn cmd_cat(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() {
        return Err("Usage: cat <file>");
    }

    let filename = args[0];
    
    // Get current process's filesystem context (mocked for now)
    let vfs_context = get_current_process_fs_context()
        .ok_or("Failed to access VFS")?;

    match vfs_context.open(filename, false) {
        Ok(inode_ref) => {
            // Read the entire content of the file into a string buffer
            let mut buf: Vec<u8> = vec![0; 1024];
            
            loop {
                match inode_ref.read(&mut buf) {
                    Some(nbytes) if nbytes > 0 => {
                        for i in 0..nbytes {
                            print!("{}", buf[i] as char);
                        }
                        
                        // Reset buffer
                        buf = vec![0; 1024];
                    },
                    
                    _ => break, // End-of-file reached or error
                };
            }

        },
        
        Err(_) => return Err(&format!("File not found: {}", filename)),
    };

    Ok(())
}

/// Execute the `ls` command to list directory contents using VFS  
pub fn cmd_ls(args: &[&str]) -> Result<(), &'static str> {
    let dir_path = if args.is_empty() { 
        String::from("/")
    } else { 
        args[0].to_string()
    };

    // Get current process's filesystem context
    let vfs_context = get_current_process_fs_context()
        .ok_or("Failed to access VFS")?;

    match vfs_context.open(&dir_path, false) {
        Ok(dir_inode_ref) => {

            if !dir_inode_ref.is_directory() { 
                return Err(&format!("Not a directory: {}", dir_path));
            }

            // Read the contents of the directory
            let entries = match dir_inode_ref.readdir() {
                Some(entries) => entries,
                
                None => return Err("Error reading directory"),
            };

            for entry in &entries {
                print!("{}", &entry.name);
                
                if matches!(entry.file_type, crate::procfs::ProcFileType::Directory)
                    || (matches!(entry.file_type, crate::procfs::ProcFileType::MemInfo) && !dir_path.ends_with("/meminfo"))
                {
                    // Add trailing slash for directories
                    print!("/");
                }
                
                println!("");  // New line after each entry
            }

        },
        
        Err(_) => return Err(&format!("Directory not found: {}", dir_path)),
    };

    Ok(())
}

/// Execute the `pwd` command to display current working directory path (cwd)
pub fn cmd_pwd(args: &[&str]) -> Result<(), &'static str> {
    
    // Validate no arguments are passed
    if !args.is_empty() { 
        return Err("Usage: pwd");
    }
    
    println!("{}", &get_current_cwd());
    Ok(())
}

// Helper function to get current process's VFS context (mocked for now)
fn get_current_process_fs_context() -> Option<crate::vfs::Vfs> {
    // In a real implementation, this would be passed from the kernel
    Some(crate::vfs::Vfs { /* mock data */ })
}

// Helper function to retrieve current working directory path string  
fn get_current_cwd() -> String {
    let cwd = "/".to_string();  // Mocked value for now
    
    return cwd;
}
