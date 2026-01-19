/// Simple shell infrastructure for RustOS
/// This module provides basic shell functionality with command parsing and execution

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Environment variables for the shell
pub struct EnvironmentVariables {
    vars: BTreeMap<String, String>,
}

impl EnvironmentVariables {
    /// Create a new environment with default variables
    pub fn new() -> Self {
        let mut vars = BTreeMap::new();

        // Initialize standard shell variables
        vars.insert("PATH".to_string(), "/bin:/usr/bin".to_string());
        vars.insert("HOME".to_string(), "/home/user".to_string());
        vars.insert("USER".to_string(), "user".to_string());
        vars.insert("SHELL".to_string(), "/bin/shell".to_string());
        vars.insert("TERM".to_string(), "xterm-256color".to_string());
        vars.insert("LANG".to_string(), "en_US.UTF-8".to_string());

        Self { vars }
    }

    /// Get a variable value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }

    /// Set a variable value
    pub fn set(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
    }

    /// Remove a variable
    pub fn unset(&mut self, key: &str) {
        self.vars.remove(key);
    }
}

/// Shell state structure
pub struct Shell {
    env: EnvironmentVariables,
}

impl Shell {
    /// Create a new shell instance
    pub fn new() -> Self {
        Self {
            env: EnvironmentVariables::new(),
        }
    }

    /// Parse and execute a command line
    pub fn execute_line(&mut self, line: &str) -> Result<(), &'static str> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        // Parse command and arguments
        let mut parts = line.split_whitespace();

        if let Some(cmd) = parts.next() {
            let args: Vec<&str> = parts.collect();
            self.execute_command(cmd, &args)
        } else {
            Ok(())
        }
    }

    /// Execute a single command with arguments
    fn execute_command(&mut self, cmd: &str, args: &[&str]) -> Result<(), &'static str> {
        match cmd {
            "echo" => self.cmd_echo(args),
            "export" => self.cmd_export(args),
            "unset" => self.cmd_unset(args),
            "clear" | "cls" => self.cmd_clear(),
            "exit" => self.cmd_exit(),
            "help" => self.cmd_help(),
            _ => {
                crate::println!("Command not found: {}", cmd);
                Err("command not found")
            }
        }
    }

    /// Echo command - print arguments
    fn cmd_echo(&mut self, args: &[&str]) -> Result<(), &'static str> {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                crate::print!(" ");
            }

            // Simple variable expansion for $VAR
            if let Some(var_name) = arg.strip_prefix('$') {
                if let Some(value) = self.env.get(var_name) {
                    crate::print!("{}", value);
                } else {
                    crate::print!("");
                }
            } else {
                crate::print!("{}", arg);
            }
        }
        crate::println!();
        Ok(())
    }

    /// Export command - set environment variable
    fn cmd_export(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            // List all variables
            for (key, value) in &self.env.vars {
                crate::println!("{}={}", key, value);
            }
        } else {
            for arg in args {
                if let Some((key, value)) = arg.split_once('=') {
                    self.env.set(key.to_string(), value.to_string());
                } else {
                    crate::println!("export: invalid format. Use: export VAR=value");
                }
            }
        }
        Ok(())
    }

    /// Unset command - remove environment variable
    fn cmd_unset(&mut self, args: &[&str]) -> Result<(), &'static str> {
        for arg in args {
            self.env.unset(arg);
        }
        Ok(())
    }

    /// Clear screen command
    fn cmd_clear(&mut self) -> Result<(), &'static str> {
        crate::vga::WRITER.lock().clear_screen();
        Ok(())
    }

    /// Exit shell (for now just print a message)
    fn cmd_exit(&mut self) -> Result<(), &'static str> {
        crate::println!("Exit command not fully implemented (kernel continues running)");
        Ok(())
    }

    /// Help command - show available commands
    fn cmd_help(&mut self) -> Result<(), &'static str> {
        crate::println!("RustOS Shell - Available Commands:");
        crate::println!("  echo [args...]   - Print arguments to screen");
        crate::println!("  export VAR=val   - Set environment variable");
        crate::println!("  unset VAR        - Remove environment variable");
        crate::println!("  clear/cls        - Clear the screen");
        crate::println!("  help             - Show this help message");
        crate::println!("  exit             - Exit the shell");
        Ok(())
    }
}

// NOTE: Keyboard input integration is NOT implemented yet.
// The read_line_from_keyboard function below is a stub that needs proper implementation.
// It should:
// 1. Read scancodes from the PS/2 keyboard driver
// 2. Convert scancodes to ASCII characters
// 3. Handle special keys (backspace, enter, etc.)
// 4. Return a complete line when Enter is pressed

/// Read a line from the keyboard (STUB - NOT IMPLEMENTED)
///
/// This is a placeholder that needs to be connected to the keyboard driver.
/// Current status: Returns None because keyboard input is not integrated.
#[allow(dead_code)]
pub fn read_line_from_keyboard() -> Option<String> {
    // TODO: Implement keyboard input integration
    // Need to:
    // 1. Read from keyboard buffer (possibly via syscall or direct driver access)
    // 2. Convert PS/2 scancodes to ASCII
    // 3. Handle line editing (backspace, cursor movement, etc.)
    // 4. Return completed line on Enter key

    None
}

/// Run the shell loop (STUB - NOT FULLY IMPLEMENTED)
///
/// This function would normally run an interactive shell loop, but keyboard
/// input is not yet integrated with the shell infrastructure.
#[allow(dead_code)]
pub fn run_shell_loop() -> ! {
    let mut shell = Shell::new();

    crate::println!("RustOS Shell v0.1");
    crate::println!("Type 'help' for available commands");
    crate::println!();
    crate::println!("WARNING: Keyboard input not yet integrated!");
    crate::println!("Shell infrastructure is ready but needs keyboard driver connection.");

    // Demonstrate that the shell works by executing some test commands
    let _ = shell.execute_line("help");
    crate::println!();
    let _ = shell.execute_line("echo Shell infrastructure is working!");
    let _ = shell.execute_line("export TEST=hello");
    let _ = shell.execute_line("echo Test variable: $TEST");

    crate::println!();
    crate::println!("Shell loop would start here with keyboard input...");

    // For now, just loop forever
    loop {
        core::hint::spin_loop();
    }
}
