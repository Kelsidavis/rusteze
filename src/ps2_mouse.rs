use spin::Mutex;
use x86_64::instructions::port::Port;

/// PS/2 controller I/O ports (shared with keyboard)
const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const COMMAND_PORT: u16 = 0x64;

/// PS/2 controller commands
const CMD_ENABLE_AUX: u8 = 0xA8;       // Enable auxiliary device (mouse)
const CMD_GET_COMPAQ_STATUS: u8 = 0x20; // Read controller configuration byte
const CMD_SET_COMPAQ_STATUS: u8 = 0x60; // Write controller configuration byte
const CMD_WRITE_MOUSE: u8 = 0xD4;       // Write next byte to mouse

/// Mouse commands
const MOUSE_CMD_SET_DEFAULTS: u8 = 0xF6;
const MOUSE_CMD_ENABLE_STREAMING: u8 = 0xF4;

/// Mouse packet state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PacketState {
    WaitingForByte1,
    WaitingForByte2,
    WaitingForByte3,
}

/// Mouse button state
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

/// Mouse movement event
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub buttons: MouseButtons,
    pub x_movement: i16,
    pub y_movement: i16,
}

/// PS/2 Mouse driver
pub struct Mouse {
    data_port: Port<u8>,
    status_port: Port<u8>,
    command_port: Port<u8>,
    packet_state: PacketState,
    packet: [u8; 3],
}

impl Mouse {
    /// Create a new mouse driver
    ///
    /// # Safety
    /// Caller must ensure no other code conflicts with PS/2 port access.
    pub const unsafe fn new() -> Self {
        Mouse {
            data_port: Port::new(DATA_PORT),
            status_port: Port::new(STATUS_PORT),
            command_port: Port::new(COMMAND_PORT),
            packet_state: PacketState::WaitingForByte1,
            packet: [0; 3],
        }
    }

    /// Wait until the controller input buffer is empty (ready to receive commands)
    fn wait_for_write(&mut self) {
        for _ in 0..100_000 {
            let status = unsafe { self.status_port.read() };
            if (status & 0x02) == 0 {
                return;
            }
        }
    }

    /// Wait until the controller output buffer has data (ready to read)
    fn wait_for_read(&mut self) -> bool {
        for _ in 0..100_000 {
            let status = unsafe { self.status_port.read() };
            if (status & 0x01) != 0 {
                return true;
            }
        }
        false
    }

    /// Send a command to the PS/2 controller
    fn send_controller_command(&mut self, cmd: u8) {
        self.wait_for_write();
        unsafe {
            self.command_port.write(cmd);
        }
    }

    /// Send a command to the mouse (via controller)
    fn send_mouse_command(&mut self, cmd: u8) {
        self.send_controller_command(CMD_WRITE_MOUSE);
        self.wait_for_write();
        unsafe {
            self.data_port.write(cmd);
        }
        // Wait for ACK
        if self.wait_for_read() {
            unsafe {
                let _ = self.data_port.read();
            }
        }
    }

    /// Initialize the PS/2 mouse
    pub fn init(&mut self) {
        // Enable the auxiliary device (mouse)
        self.send_controller_command(CMD_ENABLE_AUX);

        // Get current controller configuration
        self.send_controller_command(CMD_GET_COMPAQ_STATUS);
        if !self.wait_for_read() {
            return;
        }
        let status = unsafe { self.data_port.read() };

        // Enable IRQ12 (bit 1) and keep other settings
        let new_status = (status | 0x02) & !0x20; // Enable IRQ12, clear mouse clock disable
        self.send_controller_command(CMD_SET_COMPAQ_STATUS);
        self.wait_for_write();
        unsafe {
            self.data_port.write(new_status);
        }

        // Set mouse to default settings
        self.send_mouse_command(MOUSE_CMD_SET_DEFAULTS);

        // Enable data streaming
        self.send_mouse_command(MOUSE_CMD_ENABLE_STREAMING);
    }

    /// Process a byte from the mouse, returns an event when a full packet is received
    pub fn process_byte(&mut self, byte: u8) -> Option<MouseEvent> {
        match self.packet_state {
            PacketState::WaitingForByte1 => {
                // First byte must have bit 3 set (always 1 in standard PS/2 mouse protocol)
                if (byte & 0x08) != 0 {
                    self.packet[0] = byte;
                    self.packet_state = PacketState::WaitingForByte2;
                }
                None
            }
            PacketState::WaitingForByte2 => {
                self.packet[1] = byte;
                self.packet_state = PacketState::WaitingForByte3;
                None
            }
            PacketState::WaitingForByte3 => {
                self.packet[2] = byte;
                self.packet_state = PacketState::WaitingForByte1;

                // Parse the complete packet
                let status = self.packet[0];
                let x_raw = self.packet[1];
                let y_raw = self.packet[2];

                let buttons = MouseButtons {
                    left: (status & 0x01) != 0,
                    right: (status & 0x02) != 0,
                    middle: (status & 0x04) != 0,
                };

                // Apply sign extension based on overflow and sign bits
                let x_movement = if (status & 0x10) != 0 {
                    // X sign bit set - negative movement
                    x_raw as i16 - 256
                } else {
                    x_raw as i16
                };

                let y_movement = if (status & 0x20) != 0 {
                    // Y sign bit set - negative movement
                    y_raw as i16 - 256
                } else {
                    y_raw as i16
                };

                Some(MouseEvent {
                    buttons,
                    x_movement,
                    y_movement,
                })
            }
        }
    }

    /// Read a byte from the data port
    pub fn read_byte(&mut self) -> u8 {
        unsafe { self.data_port.read() }
    }
}

/// Global mouse instance
pub static MOUSE: Mutex<Mouse> = Mutex::new(unsafe { Mouse::new() });

/// Initialize the PS/2 mouse
pub fn init_mouse() {
    let mut mouse = MOUSE.lock();
    mouse.init();
    crate::serial_println!("PS/2 mouse driver initialized");
}

/// Mouse interrupt handler (called from IDT for IRQ12)
pub fn mouse_interrupt_handler() {
    let mut mouse = MOUSE.lock();
    let byte = mouse.read_byte();

    if let Some(event) = mouse.process_byte(byte) {
        // Log mouse events for debugging
        if event.x_movement != 0 || event.y_movement != 0 {
            crate::serial_println!(
                "Mouse: dx={}, dy={}",
                event.x_movement,
                event.y_movement
            );
        }
        if event.buttons.left || event.buttons.right || event.buttons.middle {
            crate::serial_println!(
                "Mouse buttons: L={} M={} R={}",
                event.buttons.left,
                event.buttons.middle,
                event.buttons.right
            );
        }
    }
}
