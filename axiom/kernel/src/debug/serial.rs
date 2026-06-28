/// Serial debug backend scaffold for ADRIAN OS on x86_64-focused bring-up.
///
/// This file establishes the structure for a real serial backend
/// without yet depending on actual hardware port I/O implementation.

/// Standard early bring-up serial port assumption for x86_64/QEMU-style
/// environments. This is a planning baseline, not yet active I/O.
pub const COM1_BASE_PORT: u16 = 0x3F8;

/// Tracks whether the early serial backend is conceptually initialized.
/// This is only a placeholder and is not yet wired to real hardware state.
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
/// Future implementation should configure the serial device for early output.
pub fn serial_debug_init() -> SerialState {
    SerialState::new()
}

/// Placeholder byte write path.
/// Future implementation should send one byte through serial port I/O.
pub fn serial_debug_write_byte(_byte: u8) {
    // no-op placeholder
}

/// Placeholder string write path.
/// Future implementation should emit bytes one by one through the early serial path.
pub fn serial_debug_write(message: &str) {
    for byte in message.as_bytes() {
        serial_debug_write_byte(*byte);
    }

    // Optional future newline policy can be added here.
}
