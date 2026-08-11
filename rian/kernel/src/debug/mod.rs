pub mod serial;

/// Emit a fixed early debug marker.
///
/// Current behavior:
/// - delegates to serial placeholder backend
/// Future behavior:
/// - may route through backend selection or structured logging path
pub fn debug_marker(message: &str) {
    serial::serial_debug_write_line(message);
}

/// Emit a fixed panic marker.
///
/// Current behavior:
/// - delegates to serial placeholder backend
/// Future behavior:
/// - may use panic-safe minimal backend path
pub fn panic_marker(message: &str) {
    serial::serial_debug_write_line(message);
}
