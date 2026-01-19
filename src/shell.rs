// src/shell.rs - Basic shell implementation with command parsing and environment variables support

use crate::fs::{FileDescriptorTable, Inode};
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct Shell {
    pub fd_table: FileDescriptorTable,
}

impl fmt::Write for Shell {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Write to stdout (fd 1)
        let mut buf = [0u8; 256];
        let len = core::cmp::min(s.len(), buf.len());
        buf[..len].copy_from_slice(&s.as_bytes()[..len]);
        
        self.fd_table
            .write(1, &buf[0..len])
            .map_err(|_| fmt::Error)
    }
}

lazy_static! {
    pub static ref ENV_VARS: Mutex<EnvironmentVariables> = Mutex::new(EnvironmentVariables::default());
}

pub struct EnvironmentVariables {
    vars: Vec<(String, String)>,
}

impl Default for EnvironmentVariables {
    fn default() -> Self {
        let mut env_vars = vec![
            ("PATH".to_string(), "/bin:/usr/bin".to_string()),
            ("HOME".to_string(), "/root".to_string()),
            ("USER".to_string(), "user".to_string()),
        ];
        
        // Add some common defaults
        for (key, value) in &[
            ("SHELL", "/bin/sh"),
            ("TERM", "xterm-256color"),
            ("LANG", "en_US.UTF-8")
        ] {
            env_vars.push((key.to_string(), value.to_string()));
        }
        
        EnvironmentVariables { vars: env_vars }
    }
}

impl fmt::Write for Shell {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Write to stdout (fd 1)
        let mut buf = [0u8; 256];
        let len = core::cmp::min(s.len(), buf.len());
        buf[..len].copy_from_slice(&s.as_bytes()[..len]);
        
        self.fd_table
            .write(1, &buf[0..len])
            .map_err(|_| fmt::Error)
    }
}

impl Shell {
    pub fn new() -> Self {
        let fd_table = FileDescriptorTable::new();
        Shell { fd_table }
    }

    // Parse a command line into tokens
    pub fn parse_command(&self, input: &str) -> Vec<String> {
        if input.trim().is_empty() {
            return vec![];
        }

        let mut args = Vec::new();
        let mut current_arg = String::new();

        for c in input.chars() {
            match c {
                ' ' | '\t' => {
                    // End of argument
                    if !current_arg.is_empty() || (args.len() == 0 && !input.trim().is_empty()) {
                        args.push(current_arg);
                        current_arg = String::new();
                    }
                },
                _ => { 
                    current_arg.push(c); 
                }
            }
        }

        // Add the last argument
        if !current_arg.is_empty() || (args.len() == 0 && input.trim().is_empty()) {
            args.push(current_arg);
        }

        args
    }

    pub fn execute_command(&self, command: &str) -> Result<(), &'static str> {
        let mut env_vars = ENV_VARS.lock();
        
        // Handle built-in commands first (echo)
        if command.starts_with("echo") && !command.trim().is_empty() {
            self.handle_echo(command);
            return Ok(());
        }

        // Check for environment variable expansion
        let expanded_command = self.expand_env_variables(&env_vars, &command);

        // Split into arguments and execute (stub)
        if command.contains("=") || 
           command.starts_with("export") ||
           command.starts_with("unset") {
            return self.handle_export_unset(command);
        }

        Ok(())
    }
    
    fn handle_echo(&self, input: &str) {
        let args = self.parse_command(input.trim());
        
        // Skip "echo" itself
        for arg in &args[1..] { 
            print!("{}", arg);  
            
            if !arg.is_empty() && (input.len() > 4 || (!input.contains(" ") && input != "echo")) {
                print!(" ");
            }
        }

        println!();
    }

    fn expand_env_variables(&self, env_vars: &EnvironmentVariables, command: &str) -> String {
        let mut result = String::new();

        for c in command.chars() {
            if c == '$' && !result.is_empty() { 
                // Look for variable name
                let var_name_start_pos = result.len();
                
                while let Some(next_char) = command.get(var_name_start_pos..).and_then(|s| s.chars().next()) {
                    match next_char {
                        'a' ..= 'z' | 'A' ..= 'Z' | '_' => { 
                            // Continue building variable name
                            result.push(next_char);
                            
                            if var_name_start_pos + 1 >= command.len() || !command[var_name_start_pos+1..].chars().any(|c| c.is_alphanumeric() || c == '_') {
                                break;
                            }
                        },
                        
                        _ => { 
                            // End of variable name
                            let var_value = env_vars.get(&result);
                            
                            if let Some(value) = var_value {
                                result.push_str(&value);  
                                
                                return;  // Exit early since we've expanded the full command
                            } else {
                                break;
                            }
                        },
                    }
                }

            } else {
                result.push(c);  
            }
        }

        result
    }


    fn handle_export_unset(&self, command: &str) -> Result<(), &'static str> {        
        let mut env_vars = ENV_VARS.lock();
        
        if command.starts_with("export") || 
           (command.contains("=") && !command.trim().is_empty()) {
            
            // Handle export VAR=value
            for part in command.split_whitespace() {
                if part.contains('=') {
                    let parts: Vec<&str> = part.splitn(2, '=').collect();
                    
                    match parts.as_slice() {
                        [key, value] => { 
                            env_vars.set(key.to_string(), *value);  
                            
                            // Update PATH
                            if key == "PATH" || &*key.starts_with("PATH") {
                                let new_path = format!("{}:{}", env_vars.get(&env_vars.vars[0].1), *value);
                                env_vars.set("PATH".to_string(), new_path);
                                
                                return Ok(());
                            }
                        },
                        
                        // Handle export VAR
                        [var] => { 
                            if var == "PATH" {
                                let current_value = env_vars.get(var).unwrap_or(&String::new());
                                env_vars.set(var.to_string(), format!("{}:{}", *current_value, String::from("")));
                                
                                return Ok(());
                            } else {
                                env_vars.set(var.to_string(), String::new());  
                            }
                        }, 
                            
                        _ => return Err("Invalid syntax for export"),
                    }

                } else if part == "unset" || command.starts_with("unset") && !command.trim().is_empty() {
                    
                    // Handle unset VAR
                    let var_name = &part[5..];
                    env_vars.remove(var_name);
                
                }
            }
            
        } 
        
        Ok(())
    }

}

// Implementation of the shell loop that reads input and executes commands

pub fn run_shell_loop() -> ! { 
    println!("Welcome to RustOS Shell!");
    
    let mut shell = Shell::new();
        
    // Main command processing loop
    loop {
        print!("\n> ");
            
        if let Some(input) = read_line_from_keyboard(&mut shell.fd_table, 256) {
            match input.trim() { 
                "" => continue,
                
                "clear" | "cls" => println!("{}[H", 0x1B),
                
                // Handle built-in commands
                command if command.starts_with("echo") || !command.contains("=") &&  
                           (!&*command.is_empty()) &&
                            (input.trim().len() > 4 ||
                             input.len() == 5)
                    => {
                        shell.execute_command(command).unwrap_or_else(|e| eprintln!("{}", e));
                        
                },
                
                // Handle export and unset
                command if command.starts_with("export") || 
                              command.contains("=") && !command.trim().is_empty()
                            =>
                                {  
                                    let result = shell.handle_export_unset(&*command);
                                    
                                    match result {
                                        Ok(_) => {},
                                        
                                        Err(e) => eprintln!("{}", e),
                                    }
                                
                },
                
                // Handle exit
                "exit" | "quit" => break,
                
                _ => println!("Command not found: {}", command.trim()),
            }

        } else { 
            continue;
        }
    }
}

// Helper function to read a line from keyboard input (stub)
fn read_line_from_keyboard(fd_table: &FileDescriptorTable, max_len: usize) -> Option<String> {
    let mut buffer = vec![0u8; 256];
    
    // Read until newline or EOF
    loop { 
        match fd_table.read(0, &mut buffer[0..1]) {
            Ok(_) => break,
            
            Err(e) if e == "EOF" || e == "" => return None,

            _ => continue,
        }
        
        let c = unsafe { core::ptr::read_volatile(&buffer[0] as *const u8) };
        
        match c {
            13 | 10 => break, // Carriage return or line feed
            b'\t' => print!("    "),
            
            _ if (c >= 'a' && c <= 'z') || 
                 (c >= 'A' && c <= 'Z') ||
                  (c >= '0' && c <= '9')
                =>
                    {
                        // Add character to buffer
                        let mut new_buffer = vec![b'\x00'; 256];
                        
                        for i in 0..buffer.len() { 
                            if i < max_len - 1 || (i == max_len-1 && c != '\n') {
                                unsafe { core::ptr::write_volatile(&mut new_buffer[i] as *mut u8, buffer[i]); }
                                
                                // Print character
                                print!("{}", char::from(c));
                            
                            } else if i >= max_len - 2 || (i == max_len-1 && c != '\n') {
                                    break;
                                }

                        }


                    },
            
            _ => continue,
        }
    }

    Some(String::from_utf8_lossy(&buffer).to_string())
}
