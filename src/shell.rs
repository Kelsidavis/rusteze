use crate::{keyboard::KeyboardState, fs::FileDescriptorTable};

/// A simple environment for the shell.
pub struct EnvironmentVariables {
    pub vars: std::collections::HashMap<String, String>,
}

impl EnvironmentVariables {
    /// Create a new empty environment with default variables set.
    pub fn new() -> Self {
        let mut env = Self { 
            vars: std::collections::HashMap::new(),
        };
        
        // Initialize standard shell variables
        env.vars.insert("PATH".to_string(), "/bin:/usr/bin".to_string());
        env.vars.insert("HOME".to_string(), "/home/user".to_string());  
        env.vars.insert("USER".to_string(), "user".to_string());
        env.vars.insert("SHELL".to_string(), "/bin/shell".to_string());
        env.vars.insert("TERM".to_string(), "xterm-256color".to_string());
        env.vars.insert("LANG".to_string(), "en_US.UTF-8".to_string());

        env
    }
}

impl fmt::Write for EnvironmentVariables {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        // Write to stdout (fd 1)
        let mut buf = [0u8; 256];
        
        let len = core::cmp::min(s.len(), buf.len());
        buf[..len].copy_from_slice(&s.as_bytes()[..len]);
        
        self.vars.get("STDOUT").map_or_else(|| {
            // Use default stdout if not set
            &FileDescriptorTable::new()
        }).write(1, &buf[0..len])
    }
}

/// Read input from keyboard and parse into shell command tokens.
fn read_line_from_keyboard(fd_table: &FileDescriptorTable) -> Option<String> {
    let mut line_buffer = Vec::with_capacity(256);
    
    loop {
        // Wait for a key press
        match fd_table.read(0, &[0u8; 1]) {
            Ok(_) => { 
                if !line_buffer.is_empty() && (fd_table.read(0, &mut [0u8; 1]).is_err()) {
                    break;
                }
                
                // Read scancode from keyboard
                let mut buffer = [0u8; 256];
                match fd_table.read(0, &mut buffer) {
                    Ok(_) => { 
                        if !buffer.is_empty() && (fd_table.read(0, &[0u8]).is_err()) {
                            break;
                        }
                        
                        // Process scancode to ASCII
                        let ascii = process_scancode(&buffer);
                        
                        match ascii {
                            Some(c) => {
                                line_buffer.push(c); 
                                
                                if c == b'\n' || c == 0x0A {  
                                    break; 
                                } else {
                                    print!("{}", char::from(c));
                                }
                            },
                            
                            None => continue,
                        }

                    }, Err(_) => return Some(String::new()),
                };
            },

        // Handle backspace/delete
        if !line_buffer.is_empty() && line_buffer.last().copied() == Some(0x8) {
            print!("\x08 \x08");  // Move cursor left, space over it
            
            line_buffer.pop();
            
        } else { 
            continue;
        }
    }

    let input = String::from_utf8_lossy(&line_buffer).trim_end().to_string();

    if !input.is_empty() {
        Some(input)
    } else {
        None
    }
}

/// Process PS/2 scancode to ASCII character.
fn process_scancode(scancodes: &[u8]) -> Option<u8> { 
    // Simplified mapping - in real implementation this would be more complete
    
    match scancodes.get(0) {
        Some(&scancode @ 1..=95) => {
            let ascii = match scancode {
                2 => b'a',   // A
                3 => b'b',
                4 => b'c',
                5 => b'd',
                6 => b'e',
                7 => b'f',
                8 => b'g',
                9 => b'h',
                10=> b'i',
                11=> b'j',
                12=> b'k',
                13=> b'l', 
                14=> b'm',
                15=> b'n',
                16=> b'o',
                17=> b'p',
                18=> b'q',
                19=> b'r',
                20=> b's',
                21=> b't', 
                22=> b'u',
                23=> b'v',
                24=> b'w',
                25=> b'x',
                26=> b'y',
                27=> b'z',

                // Numbers
                29 => b'1', 
                30 => b'2',
                31 => b'3',
                32 => b'4',
                33 => b'5',
                34 => b'6',
                35=>b'7',
                36=>b'8',
                37=>b'9', 
                38=>b'0',

                // Special keys
                21=>b'\n',   // Enter key (new line)
                
                _ => return None,
            };
            
            Some(ascii)  
        },
        
        _ => None,   
    }
}

/// Parse command and execute built-in commands.
fn parse_command(line: &str, env_vars: &mut EnvironmentVariables) -> Result<(), String> {
    let mut parts = line.trim().split_whitespace();
    
    if let Some(command) = parts.next() { 
        match command.to_lowercase().as_str() {
            "echo" => echo_command(parts.collect::<Vec<_>>(), env_vars),
            
            // Built-in commands
            "help" | "?" => help_command(),
                
            "clear" | "cls" => clear_screen(),

            "exit" | "quit" => exit_shell(),

            _ => return Err(format!("Command '{}' not found. Type 'help' for available commands.", command)),
        }
    } else if line.trim().is_empty() {
        // Handle empty input
        Ok(())
        
    }

    let tokens: Vec<&str> = parts.collect();
    
    match &tokens[0][..] { 
        "echo" => echo_command(tokens, env_vars),
            
        _ => if !tokens.is_empty() && (tokens.len() > 1 || (!tokens.is_empty()))) {
            println!("Command '{}' not found. Type 'help' for available commands.", tokens[0]);
        
        } else {
            Ok(())
        }
    }

}

/// Execute the echo command with variable expansion.
fn echo_command(tokens: Vec<&str>, env_vars: &mut EnvironmentVariables) -> Result<(), String> { 
    if !tokens.is_empty() && (tokens.len() > 1 || (!tokens[0].is_ascii_alphanumeric()))) {
        for arg in tokens.iter().skip(1).map(|s| s.trim()) {
            // Handle variable expansion like $HOME or ${PATH}
            let expanded = expand_variables(arg, env_vars);
            
            print!("{}", &expanded); 
        }
        
    } else if !tokens.is_empty() && (tokens.len() > 0 || (!tokens[0].is_ascii_alphanumeric()))) {  
        println!();
    
    }

    Ok(())
}

/// Expand variables like $HOME or ${PATH} in command arguments.
fn expand_variables(text: &str, env_vars: &EnvironmentVariables) -> String {
    let mut result = text.to_string();

    // Handle variable expansion
    if text.starts_with('$') && !text.is_empty() { 
        match text.strip_prefix("$") {
            Some(name) => {
                if name == "HOME" || name == "PATH" || env_vars.vars.contains_key(&name.to_uppercase()) {
                    result = env_vars.vars.get(name).cloned().unwrap_or_default();
                    
                } else {
                    // Try to expand as ${VAR} format
                    let var_name = match text.strip_prefix("${") {  
                        Some(n) => n.trim_end_matches("}"),
                        
                        None => return String::new(),
                    };
                
                    if env_vars.vars.contains_key(var_name) {
                        result = env_vars.vars[var_name].clone();
                    
                } else {
                    // Return original string
                    text.to_string()
            }
        },
        
    }

    result 
}

/// Display help message with available commands.
fn help_command() {  
    let messages = [
        "echo <text> - Print the given text",
        "help     - Show this help message",   
        "clear    - Clear screen (cls also works)",
        "exit/quit- Terminate shell session"
    ];
    
    for msg in &messages {
        println!("{}", msg);
    }
}

/// Clear terminal screen using ANSI escape codes.
fn clear_screen() { 
    print!("{}[2J{}", 0x1B as char, 0x1B as char); // ESC [2J
}
    
/// Terminate the shell loop and exit gracefully.
fn exit_shell() -> ! {
    println!("\nExiting RustOS Shell...");
    panic!("Shell terminated");
}

// Main function to run the interactive command-line interface for userspace programs.

pub fn run_shell_loop(fd_table: &FileDescriptorTable) -> ! {

    let mut env = EnvironmentVariables::new();
    
    // Initialize environment variables
    println!("\nRustOS Shell (v0.1)");
    println!("Type 'help' to see available commands.\n");
        
        loop {
            print!("$ ");
            
            if let Some(input) = read_line_from_keyboard(fd_table) { 
                match parse_command(&input, &mut env) {
                    Ok(()) => {}, // Command executed successfully
                    Err(e) => println!("Error: {}", e),
                }
                
            } else {
                 continue;  // Skip empty input lines  
             }

        };
    }
}

// Implementation of the shell loop that reads user input and executes commands.
pub struct EnvironmentVariables { 
    pub vars: std::collections::HashMap<String, String>,
}
