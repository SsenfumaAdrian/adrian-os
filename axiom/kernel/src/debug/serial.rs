use crate::arch::x86_64::port_io::Port;

/// Serial debug backend scaffold for ADRIAN OS on x86_64-focused bring-up.

/// Standard early bring-up serial port assumption for x86_64/QEMU-style
/// environments. This is a planning baseline.
pub const COM1_BASE_PORT: u16 = 0x3F8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerialState {
    pub initialized: bool,
}

impl SerialState {
    pub const fn new() -> Self {
        Self { initialized: false }
    }
}

/// Placeholder serial init path.
///
/// Future implementation should configure the UART via port I/O.
pub fn serial_debug_init() -> SerialState {
    let _port = Port::new(COM1_BASE_PORT);
    SerialState::new()
}

/// Placeholder byte write path.
///
/// Future implementation should wait for transmitter readiness and
/// emit one byte using the port I/O abstraction.
pub fn serial_debug_write_byte(byte: u8) {
    let port = Port::new(COM1_BASE_PORT);
    let _ = byte;
    let _ = port;
}

/// Placeholder string write path.
pub fn serial_debug_write(message: &str) {
    for byte in message.as_bytes() {
        serial_debug_write_byte(*byte);
    }
}
