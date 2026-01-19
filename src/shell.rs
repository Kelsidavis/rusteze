use crate::fs::{FileDescriptorTable, Inode};
use core::fmt;

/// A simple shell environment with variables.
pub struct ShellEnvironment {
    pub vars: std::collections::HashMap<String, String>,
}

impl fmt::Write for ShellEnvironment {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        // Write to stdout (fd 1)
        let mut buf = [0u8; 256];
        let len = core::cmp::min(s.len(), buf.len());
        buf[..len].copy_from_slice(&s.as_bytes()[..len]);
        
        self.fd_table
            .write(1, &buf[0..len])
            .map_err(|_| fmt::Error)
    }
}

/// Read input from keyboard and parse into shell command tokens.
fn read_line_from_keyboard(fd_table: &FileDescriptorTable) -> Option<String> {
    let mut buffer = vec![0u8; 256];
    
    // Buffer to store the characters
    let mut line_buffer = Vec::new();
    
    loop { 
        match fd_table.read(0, &mut buffer[0..1]) {
            Ok(_) => {
                if buffer[0] == b'\n' || buffer[0] == 0x0A {
                    // End of input
                    break;
                } else if buffer[0] != 0 && line_buffer.len() < 256 - 1 { 
                    let c = buffer[0];
                    
                    match c {
                        b'\t' => {
                            // Insert four spaces for tab expansion (stub)
                            for _ in 0..4 {
                                print!(" ");
                                if line_buffer.len() >= 255 {
                                    break;
                                }
                                line_buffer.push(b' ');
                            }
                        },
                        
                        b'\x7f' | b'\b' => { 
                            // Handle backspace (delete last character)
                            let len = line_buffer.len();
                            if len > 0 && !line_buffer.is_empty() {
                                print!("\x08 \x08"); // Move cursor left, space over it
                                line_buffer.pop();  
                                
                            }
                        },
                        
                        _ => { 
                            // Append printable character to the string
                            if c >= b' ' || (c == 9) && !line_buffer.is_empty() {
                                print!("{}", char::from(c));
                                line_buffer.push(c);
                            } else if c != 0x1B {  
                                continue;
                                
                            }
                        },
                    }

                }, 
                
            Err(_) => break,
        };
    }

    // Convert buffer to string
    match String::from_utf8(line_buffer) {
        Ok(s) => Some(s),
        _ => None, 
    }
}

/// Parse a command line into tokens (words).
fn parse_command(command: &str) -> Vec<&str> {
    let mut tokens = vec![];
    
    // Split by whitespace
    for token in command.trim().split_whitespace() {
        if !token.is_empty() {  
            tokens.push(token);
        }
    }

    tokens 
}

/// Run the shell loop.
pub fn run_shell_loop(fd_table: &FileDescriptorTable) -> ! {

    let mut env = ShellEnvironment { 
            vars: std::collections::HashMap::new(),
    };
        
    // Initialize environment variables (default values)
    env.vars.insert("PATH".to_string(), "/bin:/usr/bin".to_string());
    env.vars.insert("HOME".to_string(), "/home/user".to_string());  
    env.vars.insert("USER".to_string(), "user".to_string());
    env.vars.insert("SHELL".to_string(), "/bin/shell".to_string());
    env.vars.insert("TERM".to_string(), "xterm-256color".to_string());
    env.vars.insert("LANG".to_string(), "en_US.UTF-8".to_string());

    
    // Create file descriptor table
    let mut fd_table = FileDescriptorTable::new();
        
        loop {
            print!("> ");
            
            match read_line_from_keyboard(&fd_table) { 
                Some(line) => {

                    if line.trim().is_empty() || !line.chars().any(|c| c.is_ascii_alphanumeric()) {
                        continue;
                    }
                    
                    let tokens = parse_command(&line);
                
                    // Handle built-in commands
                    match &tokens[0][..] {
                        
                        "echo" => { 
                            if tokens.len() > 1 {
                                println!("{}", line.trim_start_matches("echo ").trim());
                            } else {
                                println!();
                            }
                            
                        },
                        
                        "clear" => {
                            // Clear screen (stub)
                            print!("{}[2J", 0x1B as char);
                        },

                        "exit" | "quit" => break,

                        _ => { 
                            if tokens.len() > 0 && !tokens[0].is_empty() {

                                println!("Command '{}' not found. Type 'help' for available commands.", &tokens[0]);
                                
                            }
                            
                        },
                    }

                },

                
            None => {
                 // Handle EOF or error
                  break;
             } 
        };
    }
}

// Implementation of the shell loop that reads input and executes commands

pub struct ShellEnvironment {  
    pub vars: std::collections::HashMap<String, String>,
}
```