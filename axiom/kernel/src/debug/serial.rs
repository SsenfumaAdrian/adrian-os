use crate::arch::x86_64::port_io::Port;

/// COM1 base port assumption for early x86_64/QEMU-style bring-up.
pub const COM1_BASE_PORT: u16 = 0x3F8;

/// UART register offsets from COM1 base.
const DATA_REGISTER: u16 = 0;
const INTERRUPT_ENABLE_REGISTER: u16 = 1;
const FIFO_CONTROL_REGISTER: u16 = 2;
const LINE_CONTROL_REGISTER: u16 = 3;
const MODEM_CONTROL_REGISTER: u16 = 4;
const LINE_STATUS_REGISTER: u16 = 5;
const DIVISOR_LATCH_LOW: u16 = 0;
const DIVISOR_LATCH_HIGH: u16 = 1;

/// Transmitter Holding Register Empty bit in line status register.
const LSR_TRANSMIT_EMPTY: u8 = 1 << 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerialState {
    pub initialized: bool,
}

impl SerialState {
    pub const fn new(initialized: bool) -> Self {
        Self { initialized }
    }
}

fn com1_port(offset: u16) -> Port {
    Port::new(COM1_BASE_PORT + offset)
}

/// Initialize the early serial backend structure.
///
/// This currently performs placeholder register writes through the port
/// abstraction. Real hardware effects depend on the later actual port I/O
/// implementation.
pub fn serial_debug_init() -> SerialState {
    // Disable interrupts
    com1_port(INTERRUPT_ENABLE_REGISTER).write_u8(0x00);

    // Enable DLAB
    com1_port(LINE_CONTROL_REGISTER).write_u8(0x80);

    // Set divisor low/high bytes (placeholder baud divisor configuration)
    com1_port(DIVISOR_LATCH_LOW).write_u8(0x03);
    com1_port(DIVISOR_LATCH_HIGH).write_u8(0x00);

    // 8 bits, no parity, one stop bit
    com1_port(LINE_CONTROL_REGISTER).write_u8(0x03);

    // Enable FIFO, clear queues, placeholder threshold config
    com1_port(FIFO_CONTROL_REGISTER).write_u8(0xC7);

    // IRQs disabled, RTS/DSR set placeholder
    com1_port(MODEM_CONTROL_REGISTER).write_u8(0x03);

    SerialState::new(true)
}

fn transmitter_ready() -> bool {
    let status = com1_port(LINE_STATUS_REGISTER).read_u8();
    (status & LSR_TRANSMIT_EMPTY) != 0
}

/// Emit one byte through the early serial path.
///
/// Current behavior depends on placeholder port I/O, so this is not yet
/// hardware-visible. The structure is now ready for real port enablement.
pub fn serial_debug_write_byte(byte: u8) {
    let _ready = transmitter_ready();
    com1_port(DATA_REGISTER).write_u8(byte);
}

/// Emit a string through the early serial path.
pub fn serial_debug_write(message: &str) {
    for byte in message.as_bytes() {
        serial_debug_write_byte(*byte);
    }
}
