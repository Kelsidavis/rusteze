// src/shell.rs
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;

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
    cwd: String,
}

impl Shell {
    /// Create a new shell instance
    pub fn new() -> Self {
        Self {
            env: EnvironmentVariables::new(),
            cwd: String::from("/"),
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
            "clear/cls" => self.cmd_clear(),
            "help" => self.cmd_help(),
            "ps" => self.cmd_ps(),
            "cat" => self.cmd_cat(args),
            "ls" => self.cmd_ls(args),
            "pwd" => self.cmd_pwd(args),
            "cd" => self.cmd_cd(args),
            "mkdir" => self.cmd_mkdir(args),
            "rm" => self.cmd_rm(args),
            "reboot" => self.cmd_reboot(),
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
        crate::println!("  ps               - List running processes");
        crate::println!("  cat <file>       - Display file contents");
        crate::println!("  ls [dir]         - List directory contents");
        crate::println!("  pwd              - Print working directory");
        crate::println!("  cd <dir>         - Change directory");
        crate::println!("  mkdir <dir>       - Create directory");
        crate::println!("  rm <file>        - Remove file");
        crate::println!("  reboot           - Reboot system");
        Ok(())
    }

    /// Process list command - show running processes
    fn cmd_ps(&mut self) -> Result<(), &'static str> {
        use crate::process::PROCESS_MANAGER;

        let pm = PROCESS_MANAGER.lock();
        let count = pm.process_count();

        crate::println!("PID   STATE    NAME");
        crate::println!("---   -----    ----");
        crate::println!("  0   Running  idle");

        if count > 1 {
            crate::println!("(Additional {} process(es) in queue)", count - 1);
        }

        crate::println!("Total: {} process(es)", count);

        Ok(())
    }

    /// Cat command - display file contents
    fn cmd_cat(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            crate::println!("Usage: cat <file>");
            return Err("missing file argument");
        }

        let path = self.resolve_path(args[0]);
        let tmpfs = crate::tmpfs::TMPFS.lock();

        match tmpfs.resolve_path(&path) {
            Ok(inode) => {
                if inode.file_type() == crate::vfs::FileType::Directory {
                    crate::println!("cat: {}: Is a directory", args[0]);
                    return Err("is a directory");
                }

                let mut offset = 0;
                let mut buffer = [0u8; 1024];

                loop {
                    match inode.read(offset, &mut buffer) {
                        Ok(0) => break, // EOF
                        Ok(bytes_read) => {
                            if let Ok(s) = core::str::from_utf8(&buffer[..bytes_read]) {
                                crate::print!("{}", s);
                            } else {
                                crate::println!("cat: {}: Binary file", args[0]);
                                return Err("binary file");
                            }
                            offset += bytes_read;
                        }
                        Err(e) => {
                            crate::println!("cat: {}: {}", args[0], e);
                            return Err("read error");
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                crate::println!("cat: {}: {}", args[0], e);
                Err("file not found")
            }
        }
    }

    /// Ls command - list directory contents
    fn cmd_ls(&mut self, args: &[&str]) -> Result<(), &'static str> {
        let path = if args.is_empty() {
            self.cwd.clone()
        } else {
            self.resolve_path(args[0])
        };

        let tmpfs = crate::tmpfs::TMPFS.lock();

        match tmpfs.resolve_path(&path) {
            Ok(inode) => {
                if inode.file_type() != crate::vfs::FileType::Directory {
                    crate::println!("ls: {}: Not a directory", args.get(0).unwrap_or(&"."));
                    return Err("not a directory");
                }

                match inode.list() {
                    Ok(entries) => {
                        for entry in entries {
                            // Check if it's a directory by looking it up
                            if let Ok(child_inode) = inode.lookup(&entry) {
                                if child_inode.file_type() == crate::vfs::FileType::Directory {
                                    crate::println!("{}/", entry);
                                } else {
                                    crate::println!("{}", entry);
                                }
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        crate::println!("ls: {}: {}", args.get(0).unwrap_or(&"."), e);
                        Err("read error")
                    }
                }
            }
            Err(e) => {
                crate::println!("ls: {}: {}", args.get(0).unwrap_or(&"."), e);
                Err("directory not found")
            }
        }
    }

    /// Pwd command - print working directory
    fn cmd_pwd(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if !args.is_empty() {
            crate::println!("pwd: too many arguments");
            return Err("too many arguments");
        }

        crate::println!("{}", self.cwd);
        Ok(())
    }

    /// Cd command - change directory
    fn cmd_cd(&mut self, args: &[&str]) -> Result<(), &'static str> {
        let new_path = if args.is_empty() {
            "/".to_string()
        } else {
            self.resolve_path(args[0])
        };

        let tmpfs = crate::tmpfs::TMPFS.lock();

        match tmpfs.resolve_path(&new_path) {
            Ok(inode) => {
                if inode.file_type() != crate::vfs::FileType::Directory {
                    crate::println!("cd: {}: Not a directory", args[0]);
                    return Err("not a directory");
                }
                drop(tmpfs); // Release lock before modifying self
                self.cwd = new_path;
                Ok(())
            }
            Err(e) => {
                crate::println!("cd: {}: {}", args.get(0).unwrap_or(&""), e);
                Err("directory not found")
            }
        }
    }

    /// Mkdir command - create directory
    fn cmd_mkdir(&mut self, args: &[&str]) -> Result<(), &'static str> {
        let mut vfs = crate::vfs::VFS.lock();
        let dir_path = PathBuf::from(args[0]);
        if !vfs.mkdir(&dir_path).is_ok() {
            return Err("Failed to create directory");
        }
        Ok(())
    }

    /// Rm command - remove file
    fn cmd_rm(&mut self, args: &[&str]) -> Result<(), &'static str> {
        let mut vfs = crate::vfs::VFS.lock();
        let path = PathBuf::from(args[0]);
        if !vfs.rm(&path).is_ok() {
            return Err("Failed to remove file");
        }
        Ok(())
    }

    /// Reboot command - reboot system
    fn cmd_reboot(&mut self) -> Result<(), &'static str> {
        crate::println!("Rebooting...");
        // TODO: Implement proper reboot logic
        Ok(())
    }

    /// Resolve a path (handle relative paths)
    fn resolve_path(&self, path: &str) -> String {
        if path.starts_with('/') {
            // Absolute path
            path.to_string()
        } else {
            // Relative path
            if self.cwd == "/" {
                format!("/{}", path)
            } else {
                format!("{}/{}", self.cwd, path)
            }
        }
    }
}
```

src/vfs.rs
