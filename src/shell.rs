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
    /// Command history buffer (circular, max 100 commands)
    #[allow(dead_code)]
    history: Vec<String>,
    /// Current position in history (-1 = not browsing)
    #[allow(dead_code)]
    history_pos: isize,
    /// Maximum history size
    #[allow(dead_code)]
    max_history: usize,
    /// Current input line being edited
    #[allow(dead_code)]
    current_line: String,
    /// Cursor position in current line
    #[allow(dead_code)]
    cursor_pos: usize,
}

impl Shell {
    /// Create a new shell instance
    pub fn new() -> Self {
        Self {
            env: EnvironmentVariables::new(),
            cwd: String::from("/"),
            history: Vec::new(),
            history_pos: -1,
            max_history: 100,
            current_line: String::new(),
            cursor_pos: 0,
        }
    }

    /// Add command to history
    #[allow(dead_code)]
    fn add_to_history(&mut self, cmd: &str) {
        // Don't add empty commands or duplicates of the last command
        if cmd.is_empty() {
            return;
        }

        if let Some(last) = self.history.last() {
            if last == cmd {
                return;
            }
        }

        self.history.push(cmd.to_string());

        // Keep history at max size
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Navigate history (delta: -1 for up, +1 for down)
    #[allow(dead_code)]
    pub fn history_navigate(&mut self, delta: isize) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }

        // Calculate new position
        let new_pos = if self.history_pos == -1 {
            // Start browsing from the end
            if delta < 0 {
                (self.history.len() as isize) - 1
            } else {
                return None;
            }
        } else {
            self.history_pos + delta
        };

        // Bounds checking
        if new_pos < 0 {
            // At the beginning, can't go further back
            return Some(self.history[self.history_pos as usize].clone());
        } else if new_pos >= self.history.len() as isize {
            // Past the end, return to current line
            self.history_pos = -1;
            return Some(self.current_line.clone());
        }

        self.history_pos = new_pos;
        Some(self.history[new_pos as usize].clone())
    }

    /// Move cursor left/right in current line
    #[allow(dead_code)]
    pub fn move_cursor(&mut self, delta: isize) -> usize {
        let new_pos = (self.cursor_pos as isize) + delta;
        let line_len = self.current_line.len() as isize;

        if new_pos < 0 {
            self.cursor_pos = 0;
        } else if new_pos > line_len {
            self.cursor_pos = self.current_line.len();
        } else {
            self.cursor_pos = new_pos as usize;
        }

        self.cursor_pos
    }

    /// Insert character at cursor position
    #[allow(dead_code)]
    pub fn insert_char(&mut self, ch: char) {
        self.current_line.insert(self.cursor_pos, ch);
        self.cursor_pos += 1;
    }

    /// Delete character before cursor (backspace)
    #[allow(dead_code)]
    pub fn delete_char(&mut self) -> bool {
        if self.cursor_pos > 0 {
            self.current_line.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
            true
        } else {
            false
        }
    }

    /// Get current line being edited
    #[allow(dead_code)]
    pub fn get_current_line(&self) -> &str {
        &self.current_line
    }

    /// Get cursor position
    #[allow(dead_code)]
    pub fn get_cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Clear current line
    #[allow(dead_code)]
    pub fn clear_current_line(&mut self) {
        self.current_line.clear();
        self.cursor_pos = 0;
        self.history_pos = -1;
    }

    /// Set current line (used for history navigation)
    #[allow(dead_code)]
    pub fn set_current_line(&mut self, line: String) {
        self.current_line = line;
        self.cursor_pos = self.current_line.len();
    }

    /// Complete current line and execute
    #[allow(dead_code)]
    pub fn complete_and_execute(&mut self) -> Result<(), &'static str> {
        let line = self.current_line.clone();

        // Add to history before clearing
        self.add_to_history(&line);

        // Reset line state
        self.clear_current_line();

        // Execute the command
        self.execute_line(&line)
    }

    /// Tab completion - complete command or file path
    #[allow(dead_code)]
    pub fn tab_complete(&mut self) -> Option<Vec<String>> {
        let line = &self.current_line[..self.cursor_pos];

        // Split into parts
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            // Complete command at start of line
            return self.complete_command("");
        }

        if parts.len() == 1 && !line.ends_with(' ') {
            // Still typing first word - complete command
            return self.complete_command(parts[0]);
        }

        // Completing a file/directory argument
        let to_complete = if line.ends_with(' ') {
            ""
        } else {
            parts.last().unwrap_or(&"")
        };

        self.complete_path(to_complete)
    }

    /// Complete command name
    #[allow(dead_code)]
    fn complete_command(&self, prefix: &str) -> Option<Vec<String>> {
        let commands = [
            "cat", "cd", "clear/cls", "echo", "export", "help",
            "ls", "mkdir", "ps", "pwd", "reboot", "rm", "unset",
        ];

        let matches: Vec<String> = commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|s| s.to_string())
            .collect();

        if matches.is_empty() {
            None
        } else {
            Some(matches)
        }
    }

    /// Complete file/directory path
    #[allow(dead_code)]
    fn complete_path(&self, prefix: &str) -> Option<Vec<String>> {
        // Determine the directory to search and the filename prefix
        let (search_dir, file_prefix) = if prefix.contains('/') {
            // Has directory component
            let last_slash = prefix.rfind('/').unwrap();
            let dir_part = &prefix[..=last_slash];
            let file_part = &prefix[last_slash + 1..];
            (self.resolve_path(dir_part), file_part.to_string())
        } else {
            // Just a filename in current directory
            (self.cwd.clone(), prefix.to_string())
        };

        // Try to get directory listing
        let tmpfs = crate::tmpfs::TMPFS.lock();

        if let Ok(inode) = tmpfs.resolve_path(&search_dir) {
            if let Ok(entries) = inode.list() {
                let matches: Vec<String> = entries
                    .into_iter()
                    .filter(|name| name.starts_with(&file_prefix))
                    .collect();

                if matches.is_empty() {
                    return None;
                } else {
                    return Some(matches);
                }
            }
        }

        None
    }

    /// Apply tab completion to current line
    #[allow(dead_code)]
    pub fn apply_completion(&mut self, completion: &str) {
        // Find the word to replace at cursor position
        let before_cursor = &self.current_line[..self.cursor_pos];
        let last_space = before_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);

        // Replace from last space to cursor with completion
        self.current_line.replace_range(last_space..self.cursor_pos, completion);
        self.cursor_pos = last_space + completion.len();
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
        if args.is_empty() {
            crate::println!("Usage: mkdir <directory>");
            return Err("missing directory argument");
        }

        let path = self.resolve_path(args[0]);
        let tmpfs = crate::tmpfs::TMPFS.lock();

        match tmpfs.create_directory(&path) {
            Ok(_) => {
                crate::println!("Created directory: {}", args[0]);
                Ok(())
            }
            Err(e) => {
                crate::println!("mkdir: {}: {}", args[0], e);
                Err("failed to create directory")
            }
        }
    }

    /// Rm command - remove file
    fn cmd_rm(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            crate::println!("Usage: rm <file>");
            return Err("missing file argument");
        }

        // TODO: Implement file deletion in tmpfs
        crate::println!("rm: Not yet implemented");
        crate::println!("Note: File deletion requires tmpfs.remove() method to be added");
        Err("not implemented")
    }

    /// Reboot command - reboot system
    fn cmd_reboot(&mut self) -> Result<(), &'static str> {
        crate::println!("Reboot functionality not yet implemented");
        crate::println!("Note: Requires ACPI shutdown/reset or keyboard controller reset");
        crate::println!("For now, please restart the VM manually");
        Err("not implemented")
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