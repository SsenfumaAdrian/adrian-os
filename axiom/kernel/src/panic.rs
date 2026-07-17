#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

/// Halt forever.
/// In later stages this will become architecture-aware.
pub fn halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// Kernel panic handler for no_std (bare-metal) environments.
/// Early implementation intentionally does minimal work.
///
/// Only defined when std is NOT linked: std already provides its own
/// `panic_impl` lang item, and a freestanding binary is only allowed
/// exactly one. Hosted builds (feature = "std") get theirs from std.
#[cfg(not(feature = "std"))]
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
