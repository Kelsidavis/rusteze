use crate::process::{ProcessManager, ProcessState};
use std::collections::HashMap;

/// A simple shell for interacting with the operating system.
pub struct Shell {
    /// Environment variables stored as key-value pairs.
    pub env: HashMap<String, String>,
}

impl Shell {
    /// Create a new instance of `Shell` initialized with default environment variables.
    ///
    /// The following are set:
    /// - PATH="/bin:/usr/bin"
    /// - HOME="/" (root directory)
    /// - USER="user" 
    /// - SHELL="/shell"
    /// - TERM="xterm-256color"
    pub fn new() -> Self {
        let mut env = HashMap::new();
        
        // Set default environment variables
        env.insert("PATH".to_string(), "/bin:/usr/bin".to_string());
        env.insert("HOME".to_string(), "/".to_string());
        env.insert("USER".to_string(), "user".to_string());
        env.insert("SHELL".to_string(), "/shell".to_string());
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        
        Self { env }
    }

    /// Parse and execute a command line input.
    ///
    /// This function handles:
    /// - Empty lines or comments (starting with #)
    /// - Command parsing using whitespace tokenization
    pub fn execute_line(&mut self, line: &str) -> Result<(), &'static str> {
        let line = line.trim();
        
        // Skip empty input and comment-only lines
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        // Parse command into parts (command + arguments)
        let mut parts = line.split_whitespace();

        let cmd = match parts.next() {
            Some(c) => c,
            None => return Err("Empty command"),
        };

        // Handle built-in commands
        match cmd.to_lowercase().as_str() {

            "echo" => self.cmd_echo(parts),
            
            "export" => self.cmd_export(parts),

            "unset" => self.cmd_unset(parts), 

            "clear" | "cls" => {
                println!("\x1B[2J\x1B[H"); // Clear screen and move cursor to top-left
                Ok(())
            },

            "exit" | "quit" => std::process::exit(0),
            
            "help" => self.cmd_help(),

            "ps" => self.cmd_ps(),
            
            _ => {
                println!("Unknown command: {}", cmd);
                Err("Command not found")
            }
        }

    }


    
    /// Display help text showing available built-in commands.
    fn cmd_help(&self) -> Result<(), &'static str> {

        
        let mut lines = vec![
            "Available Built-In Commands:",
            "",
            "  echo <text>       - Print the given text",
            "  export VAR=value     - Set an environment variable",
            "  unset VAR           - Remove an environment variable",
            "  clear               - Clear screen (Ctrl+L)",
            "  exit                - Terminate shell session",
            "  help                - Show this message", 
            "  ps                  - List active processes",
        ];

        
        // Sort and display the lines
        for line in &lines {
            println!("{}", line);
        }

        Ok(())
    }


    
    /// Display a list of currently running processes.
    fn cmd_ps(&self) -> Result<(), &'static str> {

        let pm = ProcessManager::new();
        

        // Get count and state info
        let process_count = pm.process_count(); 
        
        println!("  PID   State       Command");
        println!("-----   -----       -------");

        if process_count == 0 {
            println!("<no processes>");
            
        } else {

                for i in 1..=process_count { // Simulate a few PIDs
                    let state = match i % 3 {
                        0 => ProcessState::Running,
                        1 => ProcessState::Ready, 
                        _ => ProcessState::Blocked,
                    };

                    
                    

                    println!(" {:4}   {:9?}     [process-{}]", i, state, i);
                }
        }

        
        

        Ok(())
    }


    
    

    
    
    /// Parse and execute the `echo` command.
    ///
    /// This supports variable expansion using `$VAR`.
    fn cmd_echo(&self, mut parts: std::str::SplitWhitespace) -> Result<(), &'static str> {
        let text = match parts.next() {
            Some(t) => t,
            None => return Ok(()),
        };

        
        // Expand variables like $HOME
        let expanded_text = self.expand_variables(text);
        

        println!("{}", &expanded_text);

        Ok(())
    }

    
    

    
    
    /// Parse and execute the `export` command.
    ///
    /// Sets an environment variable to a given value. 
    fn cmd_export(&mut self, mut parts: std::str::SplitWhitespace) -> Result<(), &'static str> {
        
        let var_value = match parts.next() {
            Some(vv) => vv,
            None => return Err("Missing VAR=value"),
        };

        

        // Split on '=' to extract variable name and value
        if !var_value.contains('=') { 
                return Err("Invalid format: expected VAR=VALUE");
            
        }

        
        let mut split = var_value.splitn(2, '='); 

        let key = match split.next() {
            Some(k) => k,
            None => unreachable!(),
        };

        

        // Extract value (may be empty)
        let val = if let Some(v) = split.next() { 
                    v
                } else {

                        ""
        
                };


        


        self.env.insert(key.to_string(), val.to_string());

        Ok(())
    }

    
    

    
    
    /// Parse and execute the `unset` command.
    ///
    /// Removes an environment variable from storage.  
    fn cmd_unset(&mut self, mut parts: std::str::SplitWhitespace) -> Result<(), &'static str> {
        
        let var = match parts.next() { 
            Some(v) => v,
            None => return Err("Missing VAR"),
            
        };

        

        if !self.env.remove(var).is_some() {

                println!("unset: {}: not found", var);
                
                return Ok(());
            
            
        }

        


        
        // Remove any trailing '=' from the variable name
        let mut key = String::from(var);

        self.env.remove(&key); 

        

        Ok(())
    }


    
    

    
    
    /// Expand environment variables in a string.
    ///
    /// Replaces $VAR with its value if defined, otherwise leaves unchanged.
    fn expand_variables(&self, text: &str) -> String {
        
        let mut result = String::new();
        let mut i = 0;

        

        while i < text.len() {

            // Look for a variable expansion
            if text.as_bytes()[i] == b'$' && (i + 1) < text.len() { 

                // Find the end of this var name

                
                let start_idx = i+1;
                

                let mut j = start_idx;

                while j < text.len() {
                    match text.bytes().nth(j).unwrap_or(0) {

                        b'a'..=b'z'|b'A'..=b'Z'|b'_'|b'0'..=b'9' => { 
                            // Valid character in variable name
                            
                            j += 1;
                        
                        },
                    
                        _ => break, 

                    }
                }

                
                

                let var_name = &text[start_idx:j];

                


                if !var_name.is_empty() {

                    

                    match self.env.get(var_name) {
                         Some(val) => result.push_str(&val),
                         
                         None => { 
                             // If not found in env, keep the $VAR as-is
                             
                            for c in var_name.chars() {
                                result.push(c);
                            
                        }
                        
                    
                }

            } else {

                    i += 1;
                
            
    }



        
        


        if j > start_idx { 
             // We consumed a variable expansion

             
              i = j; 

              
         
        
      } else {


           let c = text.chars().nth(i).unwrap_or(' ');
           
            result.push(c);
          
            i += 1;
            
       }


    }

        

        
        


        if !result.is_empty() {
             return Ok(result); 
         
     }
    
    

    


    // Return the expanded string
    Ok(text.to_string())
}

}
```