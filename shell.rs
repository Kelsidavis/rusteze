use crate::fs::{Inode, File};
use core::fmt;

/// A simple shell environment with built-in commands.
pub struct Shell {
    /// The current working directory path (as a string).
    pub cwd: String,
}

impl fmt::Write for Shell {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // This is just to satisfy the trait requirement
        // Actual output will be handled by shell commands.
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

/// Execute the `cat` command to display file contents
pub fn cmd_cat(args: &[&str]) -> Result<(), &'static str> {
    if args.is_empty() {
        return Err("Usage: cat <file>");
    }

    let filename = args[0];
    
    // Get current process's VFS context (this would be passed from kernel)
    let procfs = match get_current_process_fs_context() {
        Some(fs) => fs,
        None => return Err("Failed to access filesystem"),
    };
    
    // Open file for reading
    if let Ok(inode_ref) = procfs.open(filename, false) {
        
        // Read the entire content of the file into a string buffer
        let mut buf: Vec<u8> = vec![0; 1024];
        
        loop {
            match inode_ref.read(&mut buf) {
                Some(nbytes) => {
                    if nbytes == 0 { break } // End-of-file reached
                    
                    for i in 0..nbytes {
                        print!("{}", buf[i] as char);
                    }
                    
                    buf = vec![0; 1024]; // Reset buffer
                },
                
                None => return Err("Error reading file"),
            };
        }

    } else {
        return Err(&format!("File not found: {}", filename));
    };

    Ok(())
}

/// Execute the `ls` command to list directory contents (files and subdirectories)
pub fn cmd_ls(args: &[&str]) -> Result<(), &'static str> {
    let dir_path = if args.is_empty() { 
        String::from("/")
    } else { 
        args[0].to_string()
    };

    // Get current process's VFS context
    let procfs = match get_current_process_fs_context() {
        Some(fs) => fs,
        None => return Err("Failed to access filesystem"),
    };
    
    // Open the directory for reading (using Inode::readdir)
    if let Ok(dir_inode_ref) = procfs.open(&dir_path, false) {
        
        match dir_inode_ref.readdir() {
            Some(entries) => {
                for entry in entries.iter() {
                    println!("{}", &entry.name);
                    
                    // Add a trailing slash to directories
                    if matches!(entry.file_type, crate::procfs::ProcFileType::Directory)
                        || (matches!(entry.file_type, crate::procfs::ProcFileType::MemInfo) && 
                            !dir_path.ends_with("/meminfo"))
                    {
                        print!("/");
                    }
                    
                    println!(""); // New line after each entry
                }

            },
            
            None => return Err("Error reading directory"),
        };
        
    } else {
        return Err(&format!("Directory not found: {}", dir_path));
    };

    Ok(())
}

/// Execute the `pwd` command to print current working directory path (cwd)
pub fn cmd_pwd(args: &[&str]) -> Result<(), &'static str> {
    
    // Validate no arguments are passed
    if !args.is_empty() { 
        return Err("Usage: pwd");
    }
    
    println!("{}", &get_current_cwd());
    Ok(())
}

// Helper function to get current process's VFS context (mocked for now)
fn get_current_process_fs_context() -> Option<crate::fs::Vfs> {
    // In a real implementation, this would be passed from the kernel
    Some(crate::fs::Vfs { /* mock data */ })
}

// Helper function to retrieve current working directory path string
fn get_current_cwd() -> String {
    let cwd = "/".to_string();  // Mocked value for now
    
    return cwd;
}
