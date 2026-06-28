/// Early debug support for ADRIAN OS.
///
/// This module intentionally starts with a no-op backend.
/// It establishes call sites and API discipline before a real
/// serial or platform-specific early logging backend is implemented.

/// Emit a fixed early debug marker.
///
/// Current behavior:
/// - no-op placeholder
/// Future behavior:
/// - route to serial-first early output backend
pub fn debug_marker(_message: &str) {
    // no-op placeholder for early bring-up structure
}

/// Emit a fixed panic marker.
///
/// Current behavior:
/// - no-op placeholder
/// Future behavior:
/// - route to panic-safe early output backend
pub fn panic_marker(_message: &str) {
    // no-op placeholder for early bring-up structure
}
