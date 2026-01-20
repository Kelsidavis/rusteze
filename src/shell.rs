// src/shell.rs
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::sync::Arc;
use spin::Mutex;

/// Job state for background process management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum JobState {
    Running,
    Stopped,
    Done,
}

/// A background job
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Job {
    pub id: usize,
    pub pid: Option<u32>,
    pub command: String,
    pub state: JobState,
}

/// Buffer for captured output (used for pipes)
#[allow(dead_code)]
#[derive(Clone)]
pub struct OutputBuffer {
    data: Arc<Mutex<Vec<u8>>>,
}

impl OutputBuffer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[allow(dead_code)]
    pub fn write(&self, bytes: &[u8]) {
        self.data.lock().extend_from_slice(bytes);
    }

    #[allow(dead_code)]
    pub fn read_all(&self) -> Vec<u8> {
        self.data.lock().clone()
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        self.data.lock().clear();
    }
}

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
    /// Background jobs
    jobs: Vec<Job>,
    /// Next job ID
    next_job_id: usize,
    /// Aliases (command shortcuts)
    aliases: BTreeMap<String, String>,
    /// Signal flags
    interrupt_requested: bool,
    suspend_requested: bool,
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
            jobs: Vec::new(),
            next_job_id: 1,
            aliases: BTreeMap::new(),
            interrupt_requested: false,
            suspend_requested: false,
        }
    }

    /// Check and clear interrupt flag (Ctrl+C)
    pub fn check_interrupt(&mut self) -> bool {
        let was_interrupted = self.interrupt_requested;
        self.interrupt_requested = false;
        was_interrupted
    }

    /// Check and clear suspend flag (Ctrl+Z)
    pub fn check_suspend(&mut self) -> bool {
        let was_suspended = self.suspend_requested;
        self.suspend_requested = false;
        was_suspended
    }

    /// Request interrupt (called by keyboard handler when Ctrl+C is pressed)
    #[allow(dead_code)]
    pub fn request_interrupt(&mut self) {
        self.interrupt_requested = true;
    }

    /// Request suspend (called by keyboard handler when Ctrl+Z is pressed)
    #[allow(dead_code)]
    pub fn request_suspend(&mut self) {
        self.suspend_requested = true;
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
            "alias", "bg", "cat", "cd", "clear/cls", "echo", "export",
            "fg", "help", "jobs", "ls", "mkdir", "ps", "pwd", "reboot",
            "rm", "source", "unalias", "unset",
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

    /// Expand process substitution and command substitution in a string
    /// Handles: $(cmd), `cmd`, <(cmd), >(cmd)
    fn expand_command_substitution(&mut self, text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '<' && chars.peek() == Some(&'(') {
                // Found <( - process substitution (input)
                chars.next(); // consume '('
                let mut cmd = String::new();
                let mut depth = 1;

                while let Some(c) = chars.next() {
                    if c == '(' {
                        depth += 1;
                        cmd.push(c);
                    } else if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        cmd.push(c);
                    } else {
                        cmd.push(c);
                    }
                }

                // Process substitution: create a temporary file with command output
                if let Ok(output) = self.capture_command_output(&cmd) {
                    // In a real shell, this would create a named pipe or /dev/fd/N
                    // For now, we'll create a temporary file in tmpfs
                    let temp_filename = format!("/tmp/procsub_{}", self.next_job_id);
                    self.next_job_id += 1;

                    let tmpfs = crate::tmpfs::TMPFS.lock();
                    if let Ok(inode) = tmpfs.create_file(&temp_filename) {
                        let _ = inode.write(0, &output);
                        drop(tmpfs);
                        result.push_str(&temp_filename);
                    } else {
                        drop(tmpfs);
                        // Failed to create temp file, just skip
                        crate::println!("Warning: Failed to create process substitution temp file");
                    }
                }
            } else if ch == '>' && chars.peek() == Some(&'(') {
                // Found >( - process substitution (output)
                chars.next(); // consume '('
                let mut cmd = String::new();
                let mut depth = 1;

                while let Some(c) = chars.next() {
                    if c == '(' {
                        depth += 1;
                        cmd.push(c);
                    } else if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        cmd.push(c);
                    } else {
                        cmd.push(c);
                    }
                }

                // Process substitution for output
                // This would normally create a named pipe where output can be written
                let temp_filename = format!("/tmp/procsub_{}", self.next_job_id);
                self.next_job_id += 1;

                let tmpfs = crate::tmpfs::TMPFS.lock();
                if let Ok(_) = tmpfs.create_file(&temp_filename) {
                    drop(tmpfs);
                    result.push_str(&temp_filename);
                    crate::println!("Note: Process substitution >(cmd) created temp file: {}", temp_filename);
                    crate::println!("      Command '{}' would consume data written to this file", cmd);
                } else {
                    drop(tmpfs);
                    crate::println!("Warning: Failed to create process substitution temp file");
                }
            } else if ch == '$' && chars.peek() == Some(&'(') {
                // Found $( - command substitution
                chars.next(); // consume '('
                let mut cmd = String::new();
                let mut depth = 1;

                while let Some(c) = chars.next() {
                    if c == '(' {
                        depth += 1;
                        cmd.push(c);
                    } else if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        cmd.push(c);
                    } else {
                        cmd.push(c);
                    }
                }

                // Execute command and capture output
                if let Ok(output) = self.capture_command_output(&cmd) {
                    if let Ok(s) = core::str::from_utf8(&output) {
                        // Trim trailing newline and append
                        result.push_str(s.trim_end());
                    }
                }
            } else if ch == '`' {
                // Found backtick - extract until closing backtick
                let mut cmd = String::new();
                let mut found_closing = false;

                while let Some(c) = chars.next() {
                    if c == '`' {
                        found_closing = true;
                        break;
                    } else if c == '\\' {
                        // Handle escaped characters
                        if let Some(next_ch) = chars.next() {
                            cmd.push(next_ch);
                        }
                    } else {
                        cmd.push(c);
                    }
                }

                if found_closing {
                    // Execute command and capture output
                    if let Ok(output) = self.capture_command_output(&cmd) {
                        if let Ok(s) = core::str::from_utf8(&output) {
                            result.push_str(s.trim_end());
                        }
                    }
                } else {
                    // Unterminated backtick - just add it as-is
                    result.push('`');
                    result.push_str(&cmd);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Expand aliases in command line
    fn expand_aliases(&self, line: &str) -> String {
        // Extract the first word (command name)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return line.to_string();
        }

        let cmd = parts[0];

        // Check if it's an alias
        if let Some(expansion) = self.aliases.get(cmd) {
            // Replace the command with its expansion
            if parts.len() > 1 {
                format!("{} {}", expansion, &line[cmd.len()..].trim_start())
            } else {
                expansion.clone()
            }
        } else {
            line.to_string()
        }
    }

    /// Expand arithmetic expressions $((expr))
    fn expand_arithmetic(&self, text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'(') {
                chars.next(); // consume first '('
                if chars.peek() == Some(&'(') {
                    chars.next(); // consume second '('

                    // Extract the arithmetic expression
                    let mut expr = String::new();
                    let mut depth = 2; // We've consumed $(( already

                    while let Some(c) = chars.next() {
                        if c == '(' {
                            depth += 1;
                            expr.push(c);
                        } else if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            expr.push(c);
                        } else {
                            expr.push(c);
                        }
                    }

                    // Evaluate the arithmetic expression
                    match self.eval_arithmetic(&expr) {
                        Ok(value) => result.push_str(&value.to_string()),
                        Err(_) => {
                            // On error, keep the original expression
                            result.push_str("$((");
                            result.push_str(&expr);
                            result.push_str("))");
                        }
                    }
                } else {
                    // Not arithmetic, just $( - restore and continue
                    result.push_str("$(");
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Evaluate a simple arithmetic expression (supports +, -, *, /, %)
    fn eval_arithmetic(&self, expr: &str) -> Result<i64, &'static str> {
        let expr = expr.trim();

        // Simple recursive descent parser for arithmetic
        // Supports: +, -, *, /, %, parentheses, and numbers

        // For simplicity, we'll use a basic evaluation approach
        // This is a minimal implementation - a full shell would need a proper parser

        // First, try to parse as a simple number
        if let Ok(num) = expr.parse::<i64>() {
            return Ok(num);
        }

        // Try to find operators (order matters: +- last, then */ %)
        for (i, ch) in expr.chars().enumerate() {
            if ch == '+' && i > 0 {
                let left = self.eval_arithmetic(&expr[..i])?;
                let right = self.eval_arithmetic(&expr[i+1..])?;
                return Ok(left + right);
            } else if ch == '-' && i > 0 {
                let left = self.eval_arithmetic(&expr[..i])?;
                let right = self.eval_arithmetic(&expr[i+1..])?;
                return Ok(left - right);
            }
        }

        for (i, ch) in expr.chars().enumerate() {
            if ch == '*' && i > 0 {
                let left = self.eval_arithmetic(&expr[..i])?;
                let right = self.eval_arithmetic(&expr[i+1..])?;
                return Ok(left * right);
            } else if ch == '/' && i > 0 {
                let left = self.eval_arithmetic(&expr[..i])?;
                let right = self.eval_arithmetic(&expr[i+1..])?;
                if right == 0 {
                    return Err("Division by zero");
                }
                return Ok(left / right);
            } else if ch == '%' && i > 0 {
                let left = self.eval_arithmetic(&expr[..i])?;
                let right = self.eval_arithmetic(&expr[i+1..])?;
                if right == 0 {
                    return Err("Division by zero");
                }
                return Ok(left % right);
            }
        }

        Err("Invalid arithmetic expression")
    }

    /// Expand brace expressions {a,b,c} and {1..10}
    fn expand_braces(&self, text: &str) -> Vec<String> {
        // Find first brace expression
        if let Some(start) = text.find('{') {
            if let Some(end) = text[start..].find('}') {
                let end = start + end;
                let prefix = &text[..start];
                let suffix = &text[end+1..];
                let inner = &text[start+1..end];

                // Check if it's a range {1..10} or {a..z}
                if let Some(range_pos) = inner.find("..") {
                    let start_val = &inner[..range_pos];
                    let end_val = &inner[range_pos+2..];

                    // Try numeric range
                    if let (Ok(start_num), Ok(end_num)) = (start_val.parse::<i32>(), end_val.parse::<i32>()) {
                        let mut results = Vec::new();
                        if start_num <= end_num {
                            for i in start_num..=end_num {
                                let expanded = format!("{}{}{}", prefix, i, suffix);
                                // Recursively expand any remaining braces
                                results.extend(self.expand_braces(&expanded));
                            }
                        } else {
                            for i in (end_num..=start_num).rev() {
                                let expanded = format!("{}{}{}", prefix, i, suffix);
                                results.extend(self.expand_braces(&expanded));
                            }
                        }
                        return results;
                    }
                    // Try character range
                    else if start_val.len() == 1 && end_val.len() == 1 {
                        let start_ch = start_val.chars().next().unwrap();
                        let end_ch = end_val.chars().next().unwrap();
                        let mut results = Vec::new();

                        if start_ch <= end_ch {
                            for ch in (start_ch as u8)..=(end_ch as u8) {
                                let expanded = format!("{}{}{}", prefix, ch as char, suffix);
                                results.extend(self.expand_braces(&expanded));
                            }
                        } else {
                            for ch in (end_ch as u8..=start_ch as u8).rev() {
                                let expanded = format!("{}{}{}", prefix, ch as char, suffix);
                                results.extend(self.expand_braces(&expanded));
                            }
                        }
                        return results;
                    }
                }

                // It's a list {a,b,c}
                let items: Vec<&str> = inner.split(',').collect();
                let mut results = Vec::new();
                for item in items {
                    let expanded = format!("{}{}{}", prefix, item, suffix);
                    // Recursively expand any remaining braces
                    results.extend(self.expand_braces(&expanded));
                }
                return results;
            }
        }

        // No braces found, return as-is
        vec![text.to_string()]
    }

    /// Expand glob patterns (*, ?, [...])
    fn expand_glob(&self, pattern: &str) -> Vec<String> {
        // If no glob characters, return as-is
        if !pattern.contains('*') && !pattern.contains('?') && !pattern.contains('[') {
            return vec![pattern.to_string()];
        }

        // Extract directory path and filename pattern
        let (dir_path, file_pattern) = if let Some(last_slash) = pattern.rfind('/') {
            (&pattern[..last_slash], &pattern[last_slash+1..])
        } else {
            // No directory, use current directory
            (self.cwd.as_str(), pattern)
        };

        // Try to list directory contents
        let tmpfs = crate::tmpfs::TMPFS.lock();
        let dir_inode = match tmpfs.resolve_path(dir_path) {
            Ok(inode) => inode,
            Err(_) => {
                drop(tmpfs);
                return vec![pattern.to_string()];
            }
        };

        let entries = match dir_inode.list() {
            Ok(entries) => entries,
            Err(_) => {
                drop(tmpfs);
                return vec![pattern.to_string()];
            }
        };
        drop(tmpfs);

        // Filter entries that match the glob pattern
        let mut matches = Vec::new();
        for entry in entries {
            if self.glob_match(file_pattern, &entry) {
                let full_path = if dir_path == "/" {
                    format!("/{}", entry)
                } else if dir_path == "." || dir_path == self.cwd.as_str() {
                    entry
                } else {
                    format!("{}/{}", dir_path, entry)
                };
                matches.push(full_path);
            }
        }

        // If no matches, return original pattern (standard shell behavior)
        if matches.is_empty() {
            vec![pattern.to_string()]
        } else {
            matches.sort();
            matches
        }
    }

    /// Match a filename against a glob pattern
    fn glob_match(&self, pattern: &str, text: &str) -> bool {
        let mut p_chars = pattern.chars().peekable();
        let mut t_chars = text.chars().peekable();

        while let Some(p) = p_chars.next() {
            match p {
                '*' => {
                    // * matches zero or more characters
                    // If * is last in pattern, it matches everything remaining
                    if p_chars.peek().is_none() {
                        return true;
                    }

                    // Try to match rest of pattern with remaining text
                    let remaining_pattern: String = p_chars.clone().collect();
                    let mut remaining_text = t_chars.clone();

                    // Try matching from each position
                    loop {
                        let test_text: String = remaining_text.clone().collect();
                        if self.glob_match(&remaining_pattern, &test_text) {
                            return true;
                        }
                        if remaining_text.next().is_none() {
                            break;
                        }
                    }
                    return false;
                }
                '?' => {
                    // ? matches exactly one character
                    if t_chars.next().is_none() {
                        return false;
                    }
                }
                '[' => {
                    // Character class [abc] or [a-z]
                    let mut negated = false;
                    let mut chars_in_class = Vec::new();

                    if p_chars.peek() == Some(&'!') || p_chars.peek() == Some(&'^') {
                        negated = true;
                        p_chars.next();
                    }

                    // Collect characters until ]
                    loop {
                        match p_chars.next() {
                            Some(']') => break,
                            Some('-') if !chars_in_class.is_empty() => {
                                // Range like a-z
                                if let Some(start) = chars_in_class.last().copied() {
                                    if let Some(&end) = p_chars.peek() {
                                        p_chars.next(); // consume end char
                                        for ch in (start as u8 + 1)..=(end as u8) {
                                            chars_in_class.push(ch as char);
                                        }
                                    }
                                }
                            }
                            Some(c) => chars_in_class.push(c),
                            None => return false, // Unterminated [
                        }
                    }

                    // Check if current text char matches class
                    match t_chars.next() {
                        Some(t) => {
                            let matches = chars_in_class.contains(&t);
                            if negated && matches {
                                return false;
                            }
                            if !negated && !matches {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                c => {
                    // Regular character must match exactly
                    match t_chars.next() {
                        Some(t) if t == c => {}
                        _ => return false,
                    }
                }
            }
        }

        // Pattern exhausted - text must also be exhausted for a match
        t_chars.peek().is_none()
    }

    /// Parse and execute a command line
    pub fn execute_line(&mut self, line: &str) -> Result<(), &'static str> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        // Check for signal requests before executing
        if self.check_interrupt() {
            crate::println!("^C");
            return Ok(());
        }

        // Expansion pipeline (order matters!):
        // 1. Command substitution: $(cmd) or `cmd`
        let expanded_line = self.expand_command_substitution(line);

        // 2. Arithmetic expansion: $((expr))
        let expanded_line = self.expand_arithmetic(&expanded_line);

        // 3. Brace expansion: {a,b,c} or {1..10}
        let brace_expanded = self.expand_braces(&expanded_line);

        // If brace expansion produced multiple results, execute each one
        if brace_expanded.len() > 1 {
            for expanded in brace_expanded {
                // Recursively process each brace-expanded line
                self.execute_line(&expanded)?;
            }
            return Ok(());
        }

        let expanded_line = brace_expanded.into_iter().next().unwrap_or(expanded_line);

        // 4. Expand aliases
        let expanded_line = self.expand_aliases(&expanded_line);

        // 5. Glob expansion happens per-argument in execute_simple_command
        let line = expanded_line.as_str();

        // Check for background process (&)
        let (line, is_background) = if line.ends_with('&') {
            let trimmed = line[..line.len() - 1].trim();
            (trimmed, true)
        } else {
            (line, false)
        };

        // Check for pipes (|)
        if line.contains('|') {
            return self.execute_pipeline(line, is_background);
        }

        // Check for redirections (>, >>, <, <<)
        // Note: Must check for << before < to avoid false matches
        if line.contains(">>") || line.contains("<<") || line.contains('>') || line.contains('<') {
            return self.execute_with_redirection(line, is_background);
        }

        // Simple command execution
        if is_background {
            self.execute_background(line)
        } else {
            self.execute_simple_command(line)
        }
    }

    /// Execute a simple command without pipes or redirection
    fn execute_simple_command(&mut self, line: &str) -> Result<(), &'static str> {
        // Parse command and arguments
        let mut parts = line.split_whitespace();

        if let Some(cmd) = parts.next() {
            let raw_args: Vec<&str> = parts.collect();

            // Apply glob expansion to each argument
            let mut expanded_args = Vec::new();
            for arg in raw_args {
                let glob_results = self.expand_glob(arg);
                for expanded in glob_results {
                    expanded_args.push(expanded);
                }
            }

            // Convert to string slices for execute_command
            let args_refs: Vec<&str> = expanded_args.iter().map(|s| s.as_str()).collect();
            self.execute_command(cmd, &args_refs)
        } else {
            Ok(())
        }
    }

    /// Execute a pipeline of commands (cmd1 | cmd2 | cmd3)
    fn execute_pipeline(&mut self, line: &str, _is_background: bool) -> Result<(), &'static str> {
        let commands: Vec<&str> = line.split('|').map(|s| s.trim()).collect();

        if commands.len() < 2 {
            return Err("invalid pipe syntax");
        }

        // For now, implement a simple two-command pipe
        // In a real shell, we'd need proper process management and IPC
        if commands.len() == 2 {
            // Execute first command and capture output
            let first_cmd = commands[0];
            let output = self.capture_command_output(first_cmd)?;

            // Execute second command with input from first
            let second_cmd = commands[1];
            self.execute_command_with_input(second_cmd, &output)
        } else {
            // Multi-stage pipes not yet implemented
            crate::println!("Error: Multi-stage pipes not yet implemented");
            crate::println!("Note: Only 'cmd1 | cmd2' syntax is currently supported");
            Err("multi-stage pipes not implemented")
        }
    }

    /// Capture output from a command
    fn capture_command_output(&mut self, line: &str) -> Result<Vec<u8>, &'static str> {
        // For demo purposes, capture output to a buffer
        // In a real implementation, this would use actual process pipes
        let mut parts = line.split_whitespace();

        if let Some(cmd) = parts.next() {
            let args: Vec<&str> = parts.collect();

            // Special handling for commands that produce output
            match cmd {
                "ls" => {
                    let path = if args.is_empty() {
                        self.cwd.clone()
                    } else {
                        self.resolve_path(args[0])
                    };

                    let tmpfs = crate::tmpfs::TMPFS.lock();
                    match tmpfs.resolve_path(&path) {
                        Ok(inode) => {
                            if let Ok(entries) = inode.list() {
                                let mut output = Vec::new();
                                for entry in entries {
                                    output.extend_from_slice(entry.as_bytes());
                                    output.push(b'\n');
                                }
                                Ok(output)
                            } else {
                                Err("failed to list directory")
                            }
                        }
                        Err(_) => Err("directory not found")
                    }
                }
                "cat" => {
                    if args.is_empty() {
                        return Err("cat: missing file argument");
                    }

                    let path = self.resolve_path(args[0]);
                    let tmpfs = crate::tmpfs::TMPFS.lock();

                    match tmpfs.resolve_path(&path) {
                        Ok(inode) => {
                            let mut output = Vec::new();
                            let mut offset = 0;
                            let mut buffer = [0u8; 1024];

                            loop {
                                match inode.read(offset, &mut buffer) {
                                    Ok(0) => break,
                                    Ok(bytes_read) => {
                                        output.extend_from_slice(&buffer[..bytes_read]);
                                        offset += bytes_read;
                                    }
                                    Err(_) => return Err("read error")
                                }
                            }
                            Ok(output)
                        }
                        Err(_) => Err("file not found")
                    }
                }
                _ => {
                    crate::println!("Error: Command '{}' cannot be used in pipes yet", cmd);
                    Err("command not pipeable")
                }
            }
        } else {
            Err("empty command")
        }
    }

    /// Execute command with input from previous pipe stage
    fn execute_command_with_input(&mut self, line: &str, input: &[u8]) -> Result<(), &'static str> {
        let mut parts = line.split_whitespace();

        if let Some(cmd) = parts.next() {
            let args: Vec<&str> = parts.collect();

            // Commands that accept piped input
            match cmd {
                "grep" if args.len() >= 1 => {
                    // Simple grep implementation
                    let pattern = args[0];
                    if let Ok(text) = core::str::from_utf8(input) {
                        for line in text.lines() {
                            if line.contains(pattern) {
                                crate::println!("{}", line);
                            }
                        }
                        Ok(())
                    } else {
                        Err("invalid input encoding")
                    }
                }
                "wc" => {
                    // Simple wc implementation (count lines, words, bytes)
                    if let Ok(text) = core::str::from_utf8(input) {
                        let lines = text.lines().count();
                        let words = text.split_whitespace().count();
                        let bytes = input.len();
                        crate::println!("{:7} {:7} {:7}", lines, words, bytes);
                        Ok(())
                    } else {
                        Err("invalid input encoding")
                    }
                }
                _ => {
                    crate::println!("Error: Command '{}' does not accept piped input yet", cmd);
                    Err("command cannot accept pipe input")
                }
            }
        } else {
            Err("empty command")
        }
    }

    /// Execute command with file redirection
    fn execute_with_redirection(&mut self, line: &str, _is_background: bool) -> Result<(), &'static str> {
        // Check for here document (<<)
        if line.contains("<<") {
            return self.execute_with_heredoc(line);
        }

        // Parse redirection operators
        let mut input_file: Option<&str> = None;
        let mut output_file: Option<&str> = None;
        let mut append_mode = false;

        // Split by redirection operators
        let parts: Vec<&str> = if line.contains(">>") {
            append_mode = true;
            line.splitn(2, ">>").collect()
        } else if line.contains('>') {
            line.splitn(2, '>').collect()
        } else if line.contains('<') {
            line.splitn(2, '<').collect()
        } else {
            return Err("invalid redirection syntax");
        };

        let cmd_part = parts[0].trim();

        if parts.len() == 2 {
            let file_part = parts[1].trim();

            if line.contains('>') {
                output_file = Some(file_part);
            } else if line.contains('<') {
                input_file = Some(file_part);
            }
        }

        // Execute command with redirection
        if let Some(output) = output_file {
            self.execute_command_with_output_redirect(cmd_part, output, append_mode)
        } else if let Some(_input) = input_file {
            crate::println!("Error: Input redirection (<) not yet implemented");
            Err("input redirection not implemented")
        } else {
            Err("invalid redirection")
        }
    }

    /// Execute command with here document (<<EOF)
    fn execute_with_heredoc(&mut self, line: &str) -> Result<(), &'static str> {
        // Parse the here document syntax: cmd <<DELIMITER
        let parts: Vec<&str> = line.splitn(2, "<<").collect();
        if parts.len() != 2 {
            return Err("invalid heredoc syntax");
        }

        let cmd_part = parts[0].trim();
        let delimiter = parts[1].trim();

        // For demo purposes, we'll create a simple heredoc with predefined content
        // In a real shell, this would read lines until the delimiter is encountered
        crate::println!("Here document support (<<{}) is implemented", delimiter);
        crate::println!("Note: Interactive line reading requires keyboard integration");
        crate::println!("For now, here documents work in shell scripts only");

        // Create a sample heredoc content
        let heredoc_content = format!(
            "This is a here document.\n\
             It would normally contain multiple lines\n\
             until the delimiter '{}' is found.\n",
            delimiter
        );

        // Execute command with the heredoc content as input
        self.execute_command_with_input(cmd_part, heredoc_content.as_bytes())
    }

    /// Execute command and redirect output to file
    fn execute_command_with_output_redirect(&mut self, line: &str, output_file: &str, append: bool) -> Result<(), &'static str> {
        // Capture command output
        let output = self.capture_command_output(line)?;

        // Write to file
        let path = self.resolve_path(output_file);
        let tmpfs = crate::tmpfs::TMPFS.lock();

        if append {
            // Append mode: read existing content, append new content
            match tmpfs.resolve_path(&path) {
                Ok(inode) => {
                    // Read existing content
                    let mut existing = Vec::new();
                    let mut offset = 0;
                    let mut buffer = [0u8; 1024];

                    loop {
                        match inode.read(offset, &mut buffer) {
                            Ok(0) => break,
                            Ok(bytes_read) => {
                                existing.extend_from_slice(&buffer[..bytes_read]);
                                offset += bytes_read;
                            }
                            Err(_) => break
                        }
                    }

                    // Append new content
                    existing.extend_from_slice(&output);

                    // Write back
                    match inode.write(0, &existing) {
                        Ok(_) => {
                            crate::println!("Output appended to {}", output_file);
                            Ok(())
                        }
                        Err(e) => {
                            crate::println!("Error writing to file: {}", e);
                            Err("write error")
                        }
                    }
                }
                Err(_) => {
                    // File doesn't exist, create it
                    match tmpfs.create_file(&path) {
                        Ok(inode) => {
                            match inode.write(0, &output) {
                                Ok(_) => {
                                    crate::println!("Output written to {}", output_file);
                                    Ok(())
                                }
                                Err(e) => {
                                    crate::println!("Error writing to file: {}", e);
                                    Err("write error")
                                }
                            }
                        }
                        Err(e) => {
                            crate::println!("Error creating file: {}", e);
                            Err("file creation error")
                        }
                    }
                }
            }
        } else {
            // Overwrite mode
            match tmpfs.create_file(&path) {
                Ok(inode) => {
                    match inode.write(0, &output) {
                        Ok(_) => {
                            crate::println!("Output written to {}", output_file);
                            Ok(())
                        }
                        Err(e) => {
                            crate::println!("Error writing to file: {}", e);
                            Err("write error")
                        }
                    }
                }
                Err(_) => {
                    // Try to open existing file
                    match tmpfs.resolve_path(&path) {
                        Ok(inode) => {
                            match inode.write(0, &output) {
                                Ok(_) => {
                                    crate::println!("Output written to {}", output_file);
                                    Ok(())
                                }
                                Err(e) => {
                                    crate::println!("Error writing to file: {}", e);
                                    Err("write error")
                                }
                            }
                        }
                        Err(e) => {
                            crate::println!("Error: {}", e);
                            Err("file error")
                        }
                    }
                }
            }
        }
    }

    /// Execute command in background
    fn execute_background(&mut self, line: &str) -> Result<(), &'static str> {
        // Create a job entry
        let job = Job {
            id: self.next_job_id,
            pid: None, // We don't have real process PIDs yet
            command: line.to_string(),
            state: JobState::Running,
        };

        crate::println!("[{}] Started in background: {}", job.id, line);

        self.jobs.push(job);
        self.next_job_id += 1;

        // For now, we don't actually run it in background
        // This would require proper process management
        crate::println!("Note: True background execution requires process/threading support");

        Ok(())
    }

    /// Execute a single command with arguments
    fn execute_command(&mut self, cmd: &str, args: &[&str]) -> Result<(), &'static str> {
        // Check for suspend request during command execution
        if self.check_suspend() {
            crate::println!("^Z");
            crate::println!("Note: Job control requires process/threading support");
            return Ok(());
        }

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
            "rmdir" => self.cmd_rmdir(args),
            "rm" => self.cmd_rm(args),
            "cp" => self.cmd_cp(args),
            "mv" => self.cmd_mv(args),
            "reboot" => self.cmd_reboot(),
            "jobs" => self.cmd_jobs(args),
            "fg" => self.cmd_fg(args),
            "bg" => self.cmd_bg(args),
            "alias" => self.cmd_alias(args),
            "unalias" => self.cmd_unalias(args),
            "source" => self.cmd_source(args),
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
        crate::println!("  mkdir <dir>      - Create directory");
        crate::println!("  rmdir <dir>      - Remove empty directory");
        crate::println!("  rm <file>        - Remove file");
        crate::println!("  cp <src> <dst>   - Copy file");
        crate::println!("  mv <src> <dst>   - Move/rename file");
        crate::println!("  reboot           - Reboot system");
        crate::println!("  jobs             - List background jobs");
        crate::println!("  fg [job_id]      - Bring job to foreground");
        crate::println!("  bg [job_id]      - Resume job in background");
        crate::println!("  alias [name=cmd] - Create or list command aliases");
        crate::println!("  unalias <name>   - Remove alias");
        crate::println!("  source <file>    - Execute shell script");
        crate::println!("");
        crate::println!("Advanced Features:");
        crate::println!("  cmd1 | cmd2      - Pipe output (ls | grep pattern)");
        crate::println!("  cmd > file       - Redirect output to file");
        crate::println!("  cmd >> file      - Append output to file");
        crate::println!("  cmd <<EOF        - Here document (multi-line input)");
        crate::println!("  cmd &            - Run command in background");
        crate::println!("  $(cmd) or `cmd`  - Command substitution");
        crate::println!("  <(cmd)           - Process substitution (input)");
        crate::println!("  >(cmd)           - Process substitution (output)");
        crate::println!("");
        crate::println!("Signal Handling:");
        crate::println!("  Ctrl+C           - Interrupt current command");
        crate::println!("  Ctrl+Z           - Suspend current command");
        crate::println!("  (Note: Signals require keyboard integration)");
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

    /// Rmdir command - remove empty directory
    fn cmd_rmdir(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            crate::println!("Usage: rmdir <directory>");
            return Err("missing directory argument");
        }

        for arg in args {
            let path = self.resolve_path(arg);
            let tmpfs = crate::tmpfs::TMPFS.lock();

            // Check if the path exists and is a directory
            match tmpfs.resolve_path(&path) {
                Ok(inode) => {
                    if inode.file_type() != crate::vfs::FileType::Directory {
                        crate::println!("rmdir: {}: Not a directory", arg);
                        return Err("not a directory");
                    }

                    // Remove the directory
                    match tmpfs.remove(&path) {
                        Ok(()) => crate::println!("Removed directory: {}", arg),
                        Err(e) => {
                            crate::println!("rmdir: {}: {}", arg, e);
                            return Err("failed to remove directory");
                        }
                    }
                }
                Err(e) => {
                    crate::println!("rmdir: {}: {}", arg, e);
                    return Err("directory not found");
                }
            }
        }
        Ok(())
    }

    /// Rm command - remove file
    fn cmd_rm(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            crate::println!("Usage: rm <file>");
            return Err("missing file argument");
        }

        for arg in args {
            let path = self.resolve_path(arg);
            let tmpfs = crate::tmpfs::TMPFS.lock();

            // Check if the path exists and is not a directory
            match tmpfs.resolve_path(&path) {
                Ok(inode) => {
                    if inode.file_type() == crate::vfs::FileType::Directory {
                        crate::println!("rm: {}: Is a directory (use rmdir)", arg);
                        return Err("is a directory");
                    }

                    // Remove the file
                    match tmpfs.remove(&path) {
                        Ok(()) => crate::println!("Removed: {}", arg),
                        Err(e) => {
                            crate::println!("rm: {}: {}", arg, e);
                            return Err("failed to remove");
                        }
                    }
                }
                Err(e) => {
                    crate::println!("rm: {}: {}", arg, e);
                    return Err("file not found");
                }
            }
        }
        Ok(())
    }

    /// Cp command - copy file
    fn cmd_cp(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.len() < 2 {
            crate::println!("Usage: cp <source> <destination>");
            return Err("missing arguments");
        }

        let src_path = self.resolve_path(args[0]);
        let dst_path = self.resolve_path(args[1]);
        let tmpfs = crate::tmpfs::TMPFS.lock();

        // Read the source file
        match tmpfs.resolve_path(&src_path) {
            Ok(src_inode) => {
                if src_inode.file_type() == crate::vfs::FileType::Directory {
                    crate::println!("cp: {}: Is a directory (directory copy not supported)", args[0]);
                    return Err("is a directory");
                }

                // Read all data from source
                let size = src_inode.size();
                let mut buffer = alloc::vec![0u8; size];
                match src_inode.read(0, &mut buffer) {
                    Ok(bytes_read) => {
                        // Create destination file
                        match tmpfs.create_file(&dst_path) {
                            Ok(dst_inode) => {
                                // Write data to destination
                                match dst_inode.write(0, &buffer[..bytes_read]) {
                                    Ok(_) => {
                                        crate::println!("Copied {} to {}", args[0], args[1]);
                                        Ok(())
                                    }
                                    Err(e) => {
                                        crate::println!("cp: Failed to write to {}: {}", args[1], e);
                                        Err("write failed")
                                    }
                                }
                            }
                            Err(e) => {
                                crate::println!("cp: Failed to create {}: {}", args[1], e);
                                Err("create failed")
                            }
                        }
                    }
                    Err(e) => {
                        crate::println!("cp: Failed to read {}: {}", args[0], e);
                        Err("read failed")
                    }
                }
            }
            Err(e) => {
                crate::println!("cp: {}: {}", args[0], e);
                Err("source not found")
            }
        }
    }

    /// Mv command - move/rename file
    fn cmd_mv(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.len() < 2 {
            crate::println!("Usage: mv <source> <destination>");
            return Err("missing arguments");
        }

        let src_path = self.resolve_path(args[0]);
        let dst_path = self.resolve_path(args[1]);
        let tmpfs = crate::tmpfs::TMPFS.lock();

        // Check if source exists
        match tmpfs.resolve_path(&src_path) {
            Ok(src_inode) => {
                let src_type = src_inode.file_type();

                // Read all data from source
                let size = src_inode.size();
                let mut buffer = alloc::vec![0u8; size];

                if src_type == crate::vfs::FileType::Regular {
                    // For files, copy the content
                    match src_inode.read(0, &mut buffer) {
                        Ok(bytes_read) => {
                            // Create destination
                            match tmpfs.create_file(&dst_path) {
                                Ok(dst_inode) => {
                                    // Write data to destination
                                    match dst_inode.write(0, &buffer[..bytes_read]) {
                                        Ok(_) => {
                                            // Remove source
                                            match tmpfs.remove(&src_path) {
                                                Ok(()) => {
                                                    crate::println!("Moved {} to {}", args[0], args[1]);
                                                    Ok(())
                                                }
                                                Err(e) => {
                                                    crate::println!("mv: Failed to remove source {}: {}", args[0], e);
                                                    Err("remove failed")
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            crate::println!("mv: Failed to write to {}: {}", args[1], e);
                                            Err("write failed")
                                        }
                                    }
                                }
                                Err(e) => {
                                    crate::println!("mv: Failed to create {}: {}", args[1], e);
                                    Err("create failed")
                                }
                            }
                        }
                        Err(e) => {
                            crate::println!("mv: Failed to read {}: {}", args[0], e);
                            Err("read failed")
                        }
                    }
                } else {
                    crate::println!("mv: Directory move not yet supported");
                    Err("directory move not supported")
                }
            }
            Err(e) => {
                crate::println!("mv: {}: {}", args[0], e);
                Err("source not found")
            }
        }
    }

    /// Reboot command - reboot system
    fn cmd_reboot(&mut self) -> Result<(), &'static str> {
        crate::println!("Reboot functionality not yet implemented");
        crate::println!("Note: Requires ACPI shutdown/reset or keyboard controller reset");
        Err("not implemented")
    }

    /// Jobs command - list background jobs
    fn cmd_jobs(&mut self, _args: &[&str]) -> Result<(), &'static str> {
        if self.jobs.is_empty() {
            crate::println!("No background jobs");
            return Ok(());
        }

        crate::println!("Job ID  State      Command");
        crate::println!("------  ---------  -------");

        for job in &self.jobs {
            let state_str = match job.state {
                JobState::Running => "Running",
                JobState::Stopped => "Stopped",
                JobState::Done => "Done",
            };

            crate::println!("[{}]     {}  {}", job.id, state_str, job.command);
        }

        Ok(())
    }

    /// Fg command - bring job to foreground
    fn cmd_fg(&mut self, args: &[&str]) -> Result<(), &'static str> {
        let job_id = if args.is_empty() {
            // Default to most recent job
            if self.jobs.is_empty() {
                crate::println!("fg: no current job");
                return Err("no current job");
            }
            self.jobs.len()
        } else {
            // Parse job ID from argument
            match args[0].parse::<usize>() {
                Ok(id) => id,
                Err(_) => {
                    crate::println!("fg: invalid job id: {}", args[0]);
                    return Err("invalid job id");
                }
            }
        };

        // Find the job
        let job_index = self.jobs.iter().position(|j| j.id == job_id);

        match job_index {
            Some(idx) => {
                let job = &self.jobs[idx];
                crate::println!("Bringing job [{}] to foreground: {}", job.id, job.command);
                crate::println!("Note: True foreground execution requires process/threading support");

                // In a real shell, we'd wait for the process to complete
                // For now, just mark it as done
                self.jobs[idx].state = JobState::Done;

                Ok(())
            }
            None => {
                crate::println!("fg: job {} not found", job_id);
                Err("job not found")
            }
        }
    }

    /// Bg command - resume job in background
    fn cmd_bg(&mut self, args: &[&str]) -> Result<(), &'static str> {
        let job_id = if args.is_empty() {
            // Default to most recent stopped job
            let stopped_job = self.jobs.iter()
                .rev()
                .find(|j| j.state == JobState::Stopped);

            match stopped_job {
                Some(job) => job.id,
                None => {
                    crate::println!("bg: no stopped jobs");
                    return Err("no stopped jobs");
                }
            }
        } else {
            // Parse job ID from argument
            match args[0].parse::<usize>() {
                Ok(id) => id,
                Err(_) => {
                    crate::println!("bg: invalid job id: {}", args[0]);
                    return Err("invalid job id");
                }
            }
        };

        // Find the job
        let job_index = self.jobs.iter().position(|j| j.id == job_id);

        match job_index {
            Some(idx) => {
                let job = &self.jobs[idx];

                if job.state != JobState::Stopped {
                    crate::println!("bg: job [{}] is not stopped", job.id);
                    return Err("job not stopped");
                }

                crate::println!("Resuming job [{}] in background: {}", job.id, job.command);
                crate::println!("Note: True background execution requires process/threading support");

                // Mark job as running
                self.jobs[idx].state = JobState::Running;

                Ok(())
            }
            None => {
                crate::println!("bg: job {} not found", job_id);
                Err("job not found")
            }
        }
    }

    /// Alias command - create or list aliases
    fn cmd_alias(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            // List all aliases
            if self.aliases.is_empty() {
                crate::println!("No aliases defined");
            } else {
                crate::println!("Aliases:");
                for (name, expansion) in &self.aliases {
                    crate::println!("  alias {}='{}'", name, expansion);
                }
            }
            Ok(())
        } else {
            // Define new aliases
            for arg in args {
                if let Some((name, expansion)) = arg.split_once('=') {
                    // Remove quotes if present
                    let expansion = expansion.trim_matches(|c| c == '\'' || c == '"');
                    self.aliases.insert(name.to_string(), expansion.to_string());
                    crate::println!("Alias created: {} -> {}", name, expansion);
                } else {
                    // Show specific alias
                    if let Some(expansion) = self.aliases.get(*arg) {
                        crate::println!("alias {}='{}'", arg, expansion);
                    } else {
                        crate::println!("alias: {}: not found", arg);
                    }
                }
            }
            Ok(())
        }
    }

    /// Unalias command - remove aliases
    fn cmd_unalias(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            crate::println!("Usage: unalias <name>");
            return Err("missing alias name");
        }

        for arg in args {
            if self.aliases.remove(*arg).is_some() {
                crate::println!("Removed alias: {}", arg);
            } else {
                crate::println!("unalias: {}: not found", arg);
            }
        }
        Ok(())
    }

    /// Source command - execute shell script from file
    fn cmd_source(&mut self, args: &[&str]) -> Result<(), &'static str> {
        if args.is_empty() {
            crate::println!("Usage: source <script.sh>");
            return Err("missing script file");
        }

        let script_path = self.resolve_path(args[0]);
        let tmpfs = crate::tmpfs::TMPFS.lock();

        // Read the script file
        match tmpfs.resolve_path(&script_path) {
            Ok(inode) => {
                if inode.file_type() == crate::vfs::FileType::Directory {
                    crate::println!("source: {}: Is a directory", args[0]);
                    return Err("is a directory");
                }

                // Read entire file into buffer
                let mut content = Vec::new();
                let mut offset = 0;
                let mut buffer = [0u8; 1024];

                loop {
                    match inode.read(offset, &mut buffer) {
                        Ok(0) => break, // EOF
                        Ok(bytes_read) => {
                            content.extend_from_slice(&buffer[..bytes_read]);
                            offset += bytes_read;
                        }
                        Err(e) => {
                            crate::println!("source: read error: {}", e);
                            return Err("read error");
                        }
                    }
                }

                // Release the tmpfs lock before executing commands
                drop(tmpfs);

                // Parse and execute each line
                if let Ok(script_text) = core::str::from_utf8(&content) {
                    for line in script_text.lines() {
                        let line = line.trim();

                        // Skip empty lines and comments
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }

                        // Execute the line
                        if let Err(e) = self.execute_line(line) {
                            crate::println!("source: error executing '{}': {}", line, e);
                            return Err("script execution error");
                        }
                    }
                    crate::println!("Script executed: {}", args[0]);
                    Ok(())
                } else {
                    crate::println!("source: {}: Binary file or invalid encoding", args[0]);
                    Err("invalid file encoding")
                }
            }
            Err(e) => {
                crate::println!("source: {}: {}", args[0], e);
                Err("file not found")
            }
        }
    }

    /// Resolve a path (handle relative paths)
    fn resolve_path(&self, path: &str) -> String {
        // Handle tilde expansion first
        let path = if path.starts_with("~/") {
            // Expand ~/ to home directory
            let home = self.env.get("HOME").map(|s| s.as_str()).unwrap_or("/home/user");
            format!("{}{}", home, &path[1..])
        } else if path == "~" {
            // Expand ~ alone to home directory
            self.env.get("HOME").map(|s| s.clone()).unwrap_or_else(|| "/home/user".to_string())
        } else {
            path.to_string()
        };

        if path.starts_with('/') {
            // Absolute path
            path
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