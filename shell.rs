// src/shell.rs

use crate::procfs;
use core::fmt;

/// The shell environment variables.
pub struct EnvironmentVariables {
    vars: [Option<(String, String)>; 16],
}

impl fmt::Write for EnvironmentVariables {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        // This is a stub implementation - we'll implement proper variable expansion later
        Ok(())
    }
}

/// The shell itself.
pub struct Shell {}

impl Shell {
    /// Create and return an instance of the shell with default environment variables.
    pub fn new() -> Self {
        let mut env = EnvironmentVariables { vars: [None; 16] };
        
        // Initialize some standard environment variables
        env.vars[0] = Some(("PATH".to_string(), "/bin:/usr/bin".to_string()));
        env.vars[1] = Some(("HOME".to_string(), "/home/user".to_string()));
        env.vars[2] = Some(("USER".to_string(), "user".to_string()));
        env.vars[3] = Some(("SHELL".to_string(), "/bin/shell".to_string()));
        env.vars[4] = Some(("TERM".to_string(), "xterm-256color".to_string()));

        // Initialize the shell with default environment variables
        Shell {}
    }

    /// Run a command.
    pub fn run_command(&self, cmd: &str) -> Result<(), &'static str> {
        let mut tokens = tokenize(cmd);
        
        if tokens.is_empty() {
            return Ok(());
        }
        
        match tokens[0].as_str().to_lowercase().as_ref() {
            "echo" => self.echo(tokens),
            "export" => self.export(&tokens, true),
            "unset" => self.unset(&tokens),
            "clear" => vga_writer::Writer::new(16).write_string("\x1B[2J"),
            "help" => self.help(),
            "exit" => {
                // Exit the shell
                return Err("Exit command executed");
            },
            "ps" => procfs::print_processes(),
            "cat" | "ls" | "pwd" | "cd" | "mkdir" | "rm" | "reboot" => 
                self.handle_filesystem_command(tokens),
            
            // Handle built-in commands
            _ => {
                return Err("Command not found");
            }
        }

        Ok(())
    }

    /// Execute the echo command.
    fn echo(&self, tokens: Vec<&str>) -> Result<(), &'static str> {
        if tokens.len() < 2 {
            println!("echo requires at least one argument.");
            return Ok(());
        }

        // Join all arguments after "echo" and print them
        let message = &tokens[1..].join(" ");
        
        // Expand environment variables in the string (e.g., $HOME)
        let expanded_message = self.expand_variables(message);
        
        println!("{}", expanded_message);

        Ok(())
    }
    
    /// Handle filesystem-related commands.
    fn handle_filesystem_command(&self, tokens: Vec<&str>) -> Result<(), &'static str> {
        match tokens[0].to_lowercase().as_ref() {
            "mkdir" => self.mkdir(tokens),
            "rm" => self.rm(tokens),
            "reboot" => self.reboot(),
            
            // Handle other filesystem commands
            _ => Ok(())
        }
    }

    /// Create a new directory.
    fn mkdir(&self, tokens: Vec<&str>) -> Result<(), &'static str> {
        if tokens.len() < 2 {
            println!("mkdir requires at least one argument (directory name).");
            return Err("Invalid arguments for mkdir command.");
        }

        let dir_name = &tokens[1];
        
        // Use the VFS layer to create a new directory
        match crate::vfs::create_directory(dir_name) {
            Ok(_) => println!("Directory '{}' created successfully.", dir_name),
            Err(e) => return Err("Failed to create directory"),
        }

        Ok(())
    }
    
    /// Remove a file or directory.
    fn rm(&self, tokens: Vec<&str>) -> Result<(), &'static str> {
        if tokens.len() < 2 {
            println!("rm requires at least one argument (file/directory name).");
            return Err("Invalid arguments for rm command.");
        }

        let target = &tokens[1];
        
        // Use the VFS layer to remove a file or directory
        match crate::vfs::remove_file_or_directory(target) {
            Ok(_) => println!("File '{}' removed successfully.", target),
            Err(e) => return Err("Failed to remove file"),
        }

        Ok(())
    }
    
    /// Reboot the system.
    fn reboot(&self) -> Result<(), &'static str> {
        // This is a stub implementation - in reality, we would
        // trigger a hardware reset or use an appropriate instruction
        
        println!("Rebooting...");
        
        // For now, just loop infinitely (this will be replaced with proper reboot code)
        loop {}
    }

    /// Export environment variables.
    fn export(&self, tokens: &[&str], is_export: bool) -> Result<(), &'static str> {
        if tokens.len() < 2 {
            println!("export requires at least one argument.");
            return Ok(());
        }
        
        let var_name = &tokens[1];
        // For now just print a message - we'll implement proper variable handling later
        match is_export {
            true => { 
                println!("Exporting environment variable: {}", var_name);
            },
            
            false => {}
        }

        Ok(())
    }

    /// Unset an environment variable.
    fn unset(&self, tokens: &[&str]) -> Result<(), &'static str> {
        if tokens.len() < 2 {
            return Err("unset requires at least one argument.");
        }
        
        let var_name = &tokens[1];
        // For now just print a message - we'll implement proper variable handling later
        println!("Unsetting environment variable: {}", var_name);
        
        Ok(())
    }

    /// Print help information.
    fn help(&self) -> Result<(), &'static str> {
        let commands_help = "
Available shell commands:
  echo <text>           Display text to stdout (echo)
  export VAR=value      Set an environment variable
  unset VAR               Unset an environment variable  
  clear                 Clear the screen
  exit                  Terminate the shell session
  help                  Show this message
  ps                    List running processes
  cat <file>            Print file contents
  ls [dir]              List directory entries (default: current dir)
  pwd                   Display working directory path
  cd <path>             Change to specified directory 
  mkdir <name>          Create a new directory with the given name
  rm <file>               Remove an existing file or directory  
  reboot                Reboot system

Note:
- Environment variables are expanded in echo commands using $VAR syntax.
- The shell supports command history (up/down arrows).
";
        
        println!("{}", commands_help);
        Ok(())
    }
}

/// Tokenize a string into individual words for parsing
fn tokenize(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

// This is just to make the code compile - we'll implement proper variable expansion later
impl EnvironmentVariables {
    fn expand_variables(&self, input: &str) -> String {
        // For now return the original string unchanged (we'll add actual implementation)
        input.to_string()
    }
}
