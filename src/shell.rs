use crate::{keyboard::KeyboardState, fs::FileDescriptorTable};

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
                } else if line_buffer.len() < 256 - 1 { 
                    let c = buffer[0];
                    
                    match c {
                        b'\t' => {
                            // Insert four spaces for tab expansion (stub)
                            print!("    ");
                            
                            // Add to our internal buffer
                            line_buffer.extend_from_slice(&b"    "[..]);
                        },
                        
                        b'\x7f' | b'\b' => { 
                            // Handle backspace/delete last character
                            if !line_buffer.is_empty() {
                                print!("\x08 \x08");  // Move cursor left, space over it
                                
                                line_buffer.pop();  
                            }
                            
                        },
                        
                        _ => { 
                            // Append printable characters to the string buffer
                            let is_printable = c >= b' ' && (c < 127 || c == 9); // ASCII control chars
                        
                            if is_printable {
                                print!("{}", char::from(c));
                                
                                line_buffer.push(c);
                            } else if c != 0x1B {  
                                continue;
                            }
                        },
                    }

                }, 
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

/// Parse a command line into tokens.
fn parse_command(command: &str, env_vars: &mut ShellEnvironment) -> Result<(), &'static str> {
    let mut parts = command.trim().split_whitespace();
    
    if let Some(first_token) = parts.next() {
        match first_token.to_lowercase().as_str() {
            "echo" => {
                // Handle echo with variable expansion
                let args: Vec<&str> = parts.collect();
                
                for arg in &args {
                    print!("{} ", arg);
                    
                    if !arg.is_empty() && (arg.starts_with('$') || 
                        env_vars.vars.contains_key(arg.trim_start_matches("$"))){
                        
                            // Expand variables like $HOME, ${PATH}
                            
                            let var_name = match arg.strip_prefix("$") {  
                                Some(name) => name,
                                
                                None => {
                                    if !args.is_empty() && args[0].starts_with("${"){
                                        &arg[arg.find('{').unwrap()+1..arg.len()-2]}
                                        
                                    } else {
                                        continue;
                                    }
                                    
                                },
                            };
                            
                            // Look up variable value
                            let var_value = env_vars.vars.get(var_name).cloned().unwrap_or_default();
                                
                            print!("{}", var_value);
                        }

                    }
                }
                
                println!();  // New line after echo
                
            },

            "help" => {
                help_command()
            },
            
            "clear" | "cls" => { 
                clear_screen()  
            }, 
            
            "exit" | "quit" => exit_shell(),
            
            _ => return Err("Command not found"),
        }
    } else if command.trim().is_empty() || !command.chars().any(|c| c.is_ascii_alphanumeric()) {
        
        // Handle empty input
        println!("No valid commands entered.");
        Ok(())
    
    }

    let tokens: Vec<&str> = parts.collect();
    
    match &tokens[0][..] { 
        "echo" => echo_command(tokens, env_vars),
            
        _ => if !tokens.is_empty() {
            // Handle built-in command
            println!("Command '{}' not found. Type 'help' for available commands.", tokens[0]);
        
        } else {
            Ok(())
        }
    }

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
                    
                    let tokens = parse_command(&line, &mut env);
                
                    // Handle built-in commands
                    match &tokens[0][..] { 
                        
                        "echo" => echo_command(tokens, &mut env),
                        
                        "clear" | "cls" => clear_screen(),
                            
                        "exit" | "quit" => break,

                        _ => {
                            if !tokens.is_empty() && tokens.len() > 1 {

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

// Helper functions

fn echo_command(tokens: Vec<&str>, env_vars: &mut ShellEnvironment) {
    
    if tokens.len() > 1 {

        for arg in &tokens[1..] {
            print!("{} ", arg);
            
            // Expand variables like $HOME, ${PATH}
            let var_name = match arg.strip_prefix("$") {  
                Some(name) => name,
                
                None => continue,
            };
            
            if env_vars.vars.contains_key(var_name) {

                println!("{}", &env_vars.vars[var_name]);
        
        } else {
            // Try to expand variables like ${PATH}
            let var_value = match arg.strip_prefix("${") {  
                Some(name) => name.trim_end_matches("}"),
                
                None => continue,
            };
            
            if env_vars.vars.contains_key(var_value) {

                println!("{}", &env_vars.vars[var_value]);
        
        } else {
            print!("{} ", arg);
        }
    }

}

println!();  // New line after echo
}
    
fn help_command() { 
    let commands = [
        "echo <text> - Print text",
        "help     - Show this message",  
        "clear    - Clear screen (cls also works)",
        "exit/quit- Terminate shell"
    ];
        
    for cmd in &commands {
        println!("{}", cmd);
    }
}

fn clear_screen() { 
    print!("{}[2J{}", 0x1B as char, 0x1B as char); // Clear screen
}
    
fn exit_shell() -> ! {

    loop {}
} 

// Implementation of the shell loop that reads input and executes commands

pub struct ShellEnvironment {
    pub vars: std::collections::HashMap<String, String>,
}

