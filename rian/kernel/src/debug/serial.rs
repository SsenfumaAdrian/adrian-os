use crate::arch::x86_64::port_io::Port;
use core::sync::atomic::{AtomicU32, Ordering};

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

/// Initialize early UART-style serial output.
///
/// This is now structured as the canonical early serial setup path.
pub fn serial_debug_init() -> SerialState {
    // Disable interrupts
    com1_port(INTERRUPT_ENABLE_REGISTER).write_u8(0x00);

    // Enable divisor latch
    com1_port(LINE_CONTROL_REGISTER).write_u8(0x80);

    // Baud divisor low/high bytes
    com1_port(DIVISOR_LATCH_LOW).write_u8(0x03);
    com1_port(DIVISOR_LATCH_HIGH).write_u8(0x00);

    // 8 bits, no parity, one stop bit
    com1_port(LINE_CONTROL_REGISTER).write_u8(0x03);

    // Enable FIFO, clear buffers
    com1_port(FIFO_CONTROL_REGISTER).write_u8(0xC7);

    // RTS/DSR set
    com1_port(MODEM_CONTROL_REGISTER).write_u8(0x03);

    SerialState::new(true)
}

/// Decode a line-status register byte: is the transmitter holding
/// register empty?
///
/// Split out from the port read so the bit test itself is checkable in a
/// hosted test -- there is no UART to read from there, but the decode is
/// where an off-by-one-bit mistake would actually live.
pub const fn line_status_says_ready(status: u8) -> bool {
    (status & LSR_TRANSMIT_EMPTY) != 0
}

fn transmitter_ready() -> bool {
    line_status_says_ready(com1_port(LINE_STATUS_REGISTER).read_u8())
}

/// How many `spin_loop` iterations to wait for the transmitter before
/// giving up on a byte.
///
/// Bounded on purpose. An unbounded `while !transmitter_ready() {}` is
/// the textbook form and it is a hang waiting to happen: if there is no
/// UART at 0x3F8 (normal on real hardware, as opposed to QEMU) the
/// status register reads back as a constant with the THRE bit clear, and
/// the kernel wedges on its first debug message -- before it has any
/// other way to tell you why. Debug output must never be able to take
/// down the boot it is describing.
///
/// A spin count rather than a duration because there is no timekeeping
/// this early: the PIT is not programmed and the TSC frequency is
/// unknown. At a plausible few cycles per iteration this is on the order
/// of a millisecond, comfortably longer than the ~87us a 115200-baud
/// character takes to clear.
#[cfg(not(feature = "std"))]
pub const TRANSMIT_SPIN_LIMIT: u32 = 100_000;

/// One attempt in hosted builds. `Port::read_u8` is stubbed to return 0
/// there (`in`/`out` are ring-0 instructions and would fault from a test
/// process), so readiness can never become true no matter how long the
/// wait -- and burning 100_000 iterations per byte would turn `cargo run
/// -p adrian-boot-image` into a crawl. Same code path, honest limit.
#[cfg(feature = "std")]
pub const TRANSMIT_SPIN_LIMIT: u32 = 1;

/// Bytes written while the transmitter never reported ready.
///
/// A count rather than a silent drop: `let _ = transmitter_ready();` --
/// what this code did before -- made a mangled boot log
/// indistinguishable from a clean one. Relaxed ordering is right; this
/// is a diagnostic tally with no other memory it must be ordered
/// against.
///
/// Only meaningful on bare metal. In hosted builds the stubbed port
/// never reports ready, so this counts every byte written -- which is a
/// true statement about the stub rather than a fault worth acting on.
static TRANSMIT_TIMEOUTS: AtomicU32 = AtomicU32::new(0);

pub fn transmit_timeouts() -> u32 {
    TRANSMIT_TIMEOUTS.load(Ordering::Relaxed)
}

/// Spin until `ready` reports true, up to `limit` attempts. Returns
/// whether it ever did.
///
/// Takes the readiness check as a parameter for the same reason
/// `init::run_init` takes its recorder: the interesting behavior is
/// whether it gives up, and at the right point, which is not observable
/// through a real UART. Production passes `transmitter_ready`.
fn spin_until<F: FnMut() -> bool>(limit: u32, mut ready: F) -> bool {
    for _ in 0..limit {
        if ready() {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

/// Emit one byte through early serial.
///
/// Waits (boundedly) for the transmitter holding register to drain
/// before writing. Writing without that wait -- the previous behavior --
/// overwrites a byte the UART has not shifted out yet, which is why
/// early boot output arrived mangled.
///
/// On timeout the byte is still written. By then the evidence says there
/// is no functioning UART at this port, in which case the write goes
/// nowhere harmlessly; withholding it would only guarantee that a
/// borderline-but-working UART loses output.
pub fn serial_debug_write_byte(byte: u8) {
    if !spin_until(TRANSMIT_SPIN_LIMIT, transmitter_ready) {
        TRANSMIT_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
    }

    com1_port(DATA_REGISTER).write_u8(byte);
    #[cfg(feature = "std")]
    host_echo_byte(byte);
}

/// Hosted-only: there's no real UART to observe, so mirror actual
/// message bytes to stdout as they would have gone out over COM1.
/// Deliberately separate from `serial_debug_init`'s register writes,
/// which go straight through `Port` and are never echoed here.
#[cfg(feature = "std")]
fn host_echo_byte(byte: u8) {
    use std::io::Write;
    let _ = std::io::stdout().write_all(&[byte]);
}

/// Emit a raw string through early serial.
pub fn serial_debug_write(message: &str) {
    for byte in message.as_bytes() {
        serial_debug_write_byte(*byte);
    }
}

/// Emit one terminal-friendly line through early serial.
/// Uses CRLF for broad emulator/terminal friendliness.
pub fn serial_debug_write_line(message: &str) {
    serial_debug_write(message);
    serial_debug_write_byte(b'\r');
    serial_debug_write_byte(b'\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thre_is_bit_five_of_the_line_status_register() {
        assert_eq!(LSR_TRANSMIT_EMPTY, 0b0010_0000);
        assert!(line_status_says_ready(0b0010_0000));
        assert!(line_status_says_ready(0xFF));
        assert!(!line_status_says_ready(0x00));
        // Every other bit set but THRE clear: the case a too-loose mask
        // would get wrong, and the one that matters, since a UART with
        // errors flagged still must not be written to.
        assert!(!line_status_says_ready(0b1101_1111));
    }

    #[test]
    fn spin_until_stops_as_soon_as_ready() {
        let mut calls = 0u32;
        assert!(spin_until(1000, || {
            calls += 1;
            calls == 3
        }));
        assert_eq!(calls, 3, "must not keep polling after success");
    }

    #[test]
    fn spin_until_gives_up_at_the_limit_instead_of_hanging() {
        // The whole point of the bounded wait. If this ever regresses
        // to an unbounded loop, this test hangs rather than fails --
        // which is itself the correct signal.
        let mut calls = 0u32;
        assert!(!spin_until(50, || {
            calls += 1;
            false
        }));
        assert_eq!(calls, 50, "must poll exactly `limit` times");
    }

    #[test]
    fn a_zero_limit_polls_nothing_and_reports_not_ready() {
        let mut calls = 0u32;
        assert!(!spin_until(0, || {
            calls += 1;
            true
        }));
        assert_eq!(calls, 0);
    }

    #[test]
    fn serial_init_reports_initialized() {
        // Hosted: every `Port` write is a no-op, so this asserts the
        // sequence runs to completion without faulting, not that a UART
        // was configured. Real register programming is only verifiable
        // under QEMU.
        assert!(serial_debug_init().initialized);
    }

    #[test]
    fn writing_bytes_does_not_panic_and_tallies_hosted_timeouts() {
        // `transmit_timeouts` is a global counter and other tests emit
        // bytes concurrently, so only the direction is assertable.
        let before = transmit_timeouts();
        serial_debug_write_line("rian: serial self-test");
        assert!(transmit_timeouts() >= before);
    }
}
