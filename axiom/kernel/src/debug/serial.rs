/// Serial debug backend placeholder for ADRIAN OS.
///
/// This is intentionally a no-op placeholder for now.
/// It establishes the backend boundary before real x86_64 serial
/// output implementation is introduced.

pub fn serial_debug_write(_message: &str) {
    // no-op placeholder
}
