/// Panic and halt placeholders for ADRIAN OS early bring-up.

/// Halt forever.
/// In later stages this should become an architecture-aware halt path.
pub fn halt_forever() -> ! {
    loop {}
}

/// Placeholder panic path.
/// Later work:
/// - emit debug output
/// - capture crash reason
/// - route to architecture-specific halt instruction path
pub fn panic_handler_placeholder() -> ! {
    halt_forever()
}
