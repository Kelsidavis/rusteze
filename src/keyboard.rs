use spin::Mutex;
use x86_64::instructions::port::Port;

/// PS/2 keyboard I/O ports
const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;

/// PS/2 keyboard scancode set 1 (scan code to ASCII translation)
/// Index is the scancode, value is the ASCII character (0 means no printable char)
static SCANCODE_TO_ASCII: [u8; 128] = [
    0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8,   // 0x00-0x0E
    b'\t', b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', // 0x0F-0x1C
    0, b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`',          // 0x1D-0x29
    0, b'\\', b'z', b'x', b'c', b'v', b'b', b'n', b'm', b',', b'.', b'/', 0, b'*',       // 0x2A-0x37
    0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x38-0x46
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x47-0x56
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x57-0x66
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x67-0x76
    0, 0, 0, 0, 0, 0, 0, 0, 0,                                                            // 0x77-0x7F
];

/// Shifted scancode to ASCII translation (for when Shift is held)
static SCANCODE_TO_ASCII_SHIFT: [u8; 128] = [
    0, 27, b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'_', b'+', 8,   // 0x00-0x0E
    b'\t', b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', b'{', b'}', b'\n', // 0x0F-0x1C
    0, b'A', b'S', b'D', b'F', b'G', b'H', b'J', b'K', b'L', b':', b'"', b'~',          // 0x1D-0x29
    0, b'|', b'Z', b'X', b'C', b'V', b'B', b'N', b'M', b'<', b'>', b'?', 0, b'*',       // 0x2A-0x37
    0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x38-0x46
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x47-0x56
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x57-0x66
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,                                       // 0x67-0x76
    0, 0, 0, 0, 0, 0, 0, 0, 0,                                                            // 0x77-0x7F
];

/// Special key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(u8),
    Escape,
    Backspace,
    Tab,
    Enter,
    LeftShift,
    RightShift,
    LeftCtrl,
    LeftAlt,
    CapsLock,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    NumLock,
    ScrollLock,
    Home,
    Up,
    PageUp,
    Left,
    Right,
    End,
    Down,
    PageDown,
    Insert,
    Delete,
    Unknown(u8),
}

/// Key event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(KeyCode),
    Released(KeyCode),
}

/// Keyboard state
pub struct KeyboardState {
    left_shift: bool,
    right_shift: bool,
    caps_lock: bool,
    left_ctrl: bool,
    left_alt: bool,
}

impl KeyboardState {
    pub const fn new() -> Self {
        KeyboardState {
            left_shift: false,
            right_shift: false,
            caps_lock: false,
            left_ctrl: false,
            left_alt: false,
        }
    }

    /// Check if shift is currently pressed
    pub fn shift_pressed(&self) -> bool {
        self.left_shift || self.right_shift
    }

    /// Check if we should use uppercase (shift XOR caps_lock for letters)
    #[allow(dead_code)]
    pub fn uppercase(&self) -> bool {
        self.shift_pressed() ^ self.caps_lock
    }

    /// Check if control is pressed
    #[allow(dead_code)]
    pub fn ctrl_pressed(&self) -> bool {
        self.left_ctrl
    }

    /// Check if alt is pressed
    #[allow(dead_code)]
    pub fn alt_pressed(&self) -> bool {
        self.left_alt
    }
}

/// PS/2 keyboard driver
pub struct Keyboard {
    data_port: Port<u8>,
    status_port: Port<u8>,
    state: KeyboardState,
}

impl Keyboard {
    /// Create a new keyboard driver
    ///
    /// # Safety
    /// Caller must ensure no other code accesses the PS/2 ports.
    pub const unsafe fn new() -> Self {
        Keyboard {
            data_port: Port::new(DATA_PORT),
            status_port: Port::new(STATUS_PORT),
            state: KeyboardState::new(),
        }
    }

    /// Check if data is available to read
    pub fn data_available(&mut self) -> bool {
        let status = unsafe { self.status_port.read() };
        (status & 0x01) != 0
    }

    /// Read a scancode from the keyboard
    pub fn read_scancode(&mut self) -> u8 {
        unsafe { self.data_port.read() }
    }

    /// Translate a scancode to a KeyCode
    fn scancode_to_keycode(&self, scancode: u8) -> KeyCode {
        match scancode {
            0x01 => KeyCode::Escape,
            0x0E => KeyCode::Backspace,
            0x0F => KeyCode::Tab,
            0x1C => KeyCode::Enter,
            0x1D => KeyCode::LeftCtrl,
            0x2A => KeyCode::LeftShift,
            0x36 => KeyCode::RightShift,
            0x38 => KeyCode::LeftAlt,
            0x3A => KeyCode::CapsLock,
            0x3B => KeyCode::F1,
            0x3C => KeyCode::F2,
            0x3D => KeyCode::F3,
            0x3E => KeyCode::F4,
            0x3F => KeyCode::F5,
            0x40 => KeyCode::F6,
            0x41 => KeyCode::F7,
            0x42 => KeyCode::F8,
            0x43 => KeyCode::F9,
            0x44 => KeyCode::F10,
            0x45 => KeyCode::NumLock,
            0x46 => KeyCode::ScrollLock,
            0x47 => KeyCode::Home,
            0x48 => KeyCode::Up,
            0x49 => KeyCode::PageUp,
            0x4B => KeyCode::Left,
            0x4D => KeyCode::Right,
            0x4F => KeyCode::End,
            0x50 => KeyCode::Down,
            0x51 => KeyCode::PageDown,
            0x52 => KeyCode::Insert,
            0x53 => KeyCode::Delete,
            0x57 => KeyCode::F11,
            0x58 => KeyCode::F12,
            code => {
                // Try to get a printable character
                if (code as usize) < SCANCODE_TO_ASCII.len() {
                    let ascii = if self.state.shift_pressed() {
                        SCANCODE_TO_ASCII_SHIFT[code as usize]
                    } else {
                        SCANCODE_TO_ASCII[code as usize]
                    };

                    if ascii != 0 && ascii != 27 && ascii != 8 {
                        // Handle caps lock for letters
                        let final_char = if ascii.is_ascii_alphabetic() && self.state.caps_lock {
                            if self.state.shift_pressed() {
                                ascii.to_ascii_lowercase()
                            } else {
                                ascii.to_ascii_uppercase()
                            }
                        } else {
                            ascii
                        };
                        return KeyCode::Char(final_char);
                    }
                }
                KeyCode::Unknown(code)
            }
        }
    }

    /// Process a scancode and update keyboard state
    pub fn process_scancode(&mut self, scancode: u8) -> Option<KeyEvent> {
        let is_release = (scancode & 0x80) != 0;
        let code = scancode & 0x7F;

        let keycode = self.scancode_to_keycode(code);

        // Update modifier state
        match keycode {
            KeyCode::LeftShift => self.state.left_shift = !is_release,
            KeyCode::RightShift => self.state.right_shift = !is_release,
            KeyCode::LeftCtrl => self.state.left_ctrl = !is_release,
            KeyCode::LeftAlt => self.state.left_alt = !is_release,
            KeyCode::CapsLock if !is_release => {
                self.state.caps_lock = !self.state.caps_lock;
            }
            _ => {}
        }

        if is_release {
            Some(KeyEvent::Released(keycode))
        } else {
            Some(KeyEvent::Pressed(keycode))
        }
    }

    /// Try to read and process a key event
    pub fn try_read_event(&mut self) -> Option<KeyEvent> {
        if self.data_available() {
            let scancode = self.read_scancode();
            self.process_scancode(scancode)
        } else {
            None
        }
    }

    /// Read a key event, blocking until one is available
    #[allow(dead_code)]
    pub fn read_event(&mut self) -> KeyEvent {
        loop {
            if let Some(event) = self.try_read_event() {
                return event;
            }
            core::hint::spin_loop();
        }
    }

    /// Try to read a character (only returns on printable key presses)
    pub fn try_read_char(&mut self) -> Option<char> {
        if let Some(KeyEvent::Pressed(keycode)) = self.try_read_event() {
            match keycode {
                KeyCode::Char(c) => Some(c as char),
                KeyCode::Enter => Some('\n'),
                KeyCode::Tab => Some('\t'),
                KeyCode::Backspace => Some('\x08'),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Read a character, blocking until one is available
    #[allow(dead_code)]
    pub fn read_char(&mut self) -> char {
        loop {
            if let Some(c) = self.try_read_char() {
                return c;
            }
            core::hint::spin_loop();
        }
    }
}

/// Global keyboard instance
pub static KEYBOARD: Mutex<Keyboard> = Mutex::new(unsafe { Keyboard::new() });

/// Initialize the PS/2 keyboard
pub fn init_keyboard() {
    // The keyboard is already initialized by the BIOS/bootloader
    // We just need to ensure our driver is ready
    crate::serial_println!("PS/2 keyboard driver initialized");
}

/// Keyboard interrupt handler (called from IDT)
pub fn keyboard_interrupt_handler() {
    let mut keyboard = KEYBOARD.lock();
    if let Some(event) = keyboard.try_read_event() {
        match event {
            KeyEvent::Pressed(KeyCode::Char(c)) => {
                crate::serial_println!("Key pressed: '{}'", c as char);
            }
            KeyEvent::Pressed(keycode) => {
                crate::serial_println!("Key pressed: {:?}", keycode);
            }
            KeyEvent::Released(_) => {
                // Typically we don't need to log key releases
            }
        }
    }
}
