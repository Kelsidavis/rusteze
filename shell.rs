// src/shell.rs

use crate::vfs::{Vfs, Inode};
use core::fmt;

/// A simple shell.
pub struct Shell {
    /// The current working directory path (cwd).
    pub cwd: String,
}

impl fmt::Write for Shell {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Write to both VGA and serial output
        let mut writer = Writer::new();
        
        if !s.is_empty() && !(self.cwd == "/") { 
            match crate::vfs::VFS.get_file(self.cwd.as_str()) {
                Some(file_ref) => file_ref.write(s),
                None => self.print_error("Invalid directory"),
            }
        } else {
            writer.write_string(s);
        }

        Ok(())
    }
}

impl Shell {

    /// Creates a new shell with default settings.
    pub fn new() -> Self {
        let cwd = String::from("/");
        
        // Initialize the VFS
        crate::vfs::VFS.init();
        
        Self { cwd }
    }


    #[inline]
    fn print_error(&self, msg: &str) {
        self.write_str("Error: ").unwrap(); 
        self.write_str(msg).unwrap();
        self.write_str("\n").unwrap();
    }

    
    /// Displays the contents of a file.
    pub fn cmd_cat(&mut self, args: &[&str]) -> fmt::Result {

        if args.is_empty() || (args.len()==1 && args[0].is_empty()) { 
            return Err(fmt::Error);
        }
            
        let filename = &args[0];
        
        // Check for relative path
        let full_path = self.resolve_relative_path(filename);

        match crate::vfs::VFS.get_file(&full_path) {
            Some(file_ref) => {

                if file_ref.is_directory() { 
                    return Err(fmt::Error);
                }
                
                let mut buffer: [u8; 1024] = [0; 1024];
                
                loop {
                    
                    match file_ref.read(&mut buffer, &full_path) {
                        Ok(bytes_read) => {

                            if bytes_read == 0 { break } // End of file

                            self.write_str(core::str::from_utf8_unchecked(&buffer[..bytes_read])).unwrap();
                        
                        },
                        Err(_) => return Err(fmt::Error),
                    }
                }

            }, 
            None => {
                
                let error_msg = format!("File not found: {}", filename);
                self.print_error(error_msg.as_str());
            }  
        };
        
        Ok(())
    }


    
    /// Lists the contents of a directory.
    pub fn cmd_ls(&mut self, args: &[&str]) -> fmt::Result {

        // Default to current working dir
        let path = if !args.is_empty() { &args[0] } else { "." };

        
        match crate::vfs::VFS.get_file(path) {
            Some(file_ref) => {


                if file_ref.is_directory() == false {
                    self.print_error("Not a directory");
                    return Err(fmt::Error);
                }
                
                // Get the list of entries
                let mut dir_entries = Vec::<String>::new();
        
                match crate::vfs::VFS.list_dir(path, &mut dir_entries) { 
                    
                    Ok(_) => {},  
                    Err(e) => {
                        self.print_error("Failed to read directory");
                        return Err(fmt::Error);
                    }
                
                };
            
                // Sort entries alphabetically
                dir_entries.sort();
        
                for entry in dir_entries.iter() {

                    let mut line = String::new();

                    
                    if crate::vfs::VFS.get_file(entry).unwrap().is_directory() {
                        
                        self.write_str(&format!("d{}", &entry)).unwrap(); 
                
                    } else { 

                        // Check file size
                        match crate::vfs::VFS.get_file_size(entry) {

                            Ok(size_bytes) =>  line = format!("{}",size_bytes),
                            
                            Err(_) => return Err(fmt::Error)
                        
                        }
                    
                        self.write_str(&format!(" {}", &entry)).unwrap();
                    }

                } // end for loop
                
            }, 
            None => {
                
                let error_msg = format!("Directory not found: {}", path);
                self.print_error(error_msg.as_str());
            }  
        };
        
        Ok(())
    }


    
    /// Prints the current working directory.
    pub fn cmd_pwd(&mut self) -> fmt::Result {

        // Check if cwd is valid
        match crate::vfs::VFS.get_file(self.cwd.as_str()) {
            
            Some(file_ref) => { 
                
                let mut path = String::new();
        
                for c in file_ref.name.chars() {
                    path.push(c);
                    
                    self.write_str(&path).unwrap(); // Write each character to output
                }
    
            },
            None => return Err(fmt::Error)
        }

        Ok(())
    }


    /// Resolves a relative path into an absolute one.
    fn resolve_relative_path(&self, filename: &str) -> String {
        
        if !filename.starts_with("/") { // Relative path
            
            let mut full_path = self.cwd.clone();
            
            match crate::vfs::VFS.get_file(filename).unwrap().is_directory() {

                true =>  return format!("{}/{}",full_path,filename),
                
                false => return filename.to_string(),
        
        }
    
    } else {
       return filename.to_string(); // Absolute path
   }

}
```