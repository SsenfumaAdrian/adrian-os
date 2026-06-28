use core::panic::PanicInfo;

/// Halt forever.
/// In later stages this will become architecture-aware.
pub fn halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// Kernel panic handler for no_std environments.
/// Early implementation intentionally does minimal work.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    crate::debug::panic_marker("AXIOM PANIC");
    halt_forever()
}

/// Placeholder explicit panic path for early code structure.
pub fn panic_handler_placeholder() -> ! {
    crate::debug::panic_marker("AXIOM PANIC PLACEHOLDER");
    halt_forever()
}
