#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

/// Halt this CPU forever.
///
/// On bare-metal x86_64 this parks the core with `hlt` in a loop rather
/// than spinning: `hlt` stops fetching until the next interrupt, so the
/// core stops burning power and stops heating instead of running a tight
/// loop at 100% for the rest of the machine's uptime. It is wrapped in a
/// loop because `hlt` *returns* when any interrupt arrives, and "halt
/// forever" has to survive that.
///
/// Interrupts are deliberately not disabled first. This function is
/// reached both from the panic handler and from a clean end-of-init, and
/// masking interrupts is a policy decision belonging to the caller that
/// knows which of those it is.
///
/// Hosted builds keep the spin form. `hlt` is a ring-0 instruction and
/// executing it from a userspace test process raises a general
/// protection fault, so the hosted dev loop must not run it -- the same
/// split already used by `arch::x86_64::port_io`.
pub fn halt_forever() -> ! {
    loop {
        park_core();
    }
}

/// Park the core until the next interrupt.
///
/// Two cfg'd definitions rather than `#[cfg]` on statements inside the
/// loop: the selection is the whole point here, and a reader should be
/// able to see each variant whole.
#[cfg(all(not(feature = "std"), target_arch = "x86_64"))]
fn park_core() {
    // SAFETY: `hlt` has no memory operands and no effect on program
    // state beyond suspending instruction fetch until the next
    // interrupt. Its only requirement is CPL 0, which holds: this
    // definition is compiled solely for the freestanding (no_std)
    // kernel, which by construction runs in ring 0. `nomem` and
    // `nostack` are accurate because the instruction touches neither;
    // `preserves_flags` because `hlt` writes no flags.
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Hosted builds, and any non-x86_64 target, get the spin form.
#[cfg(not(all(not(feature = "std"), target_arch = "x86_64")))]
fn park_core() {
    core::hint::spin_loop();
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
    crate::debug::panic_marker("RIAN PANIC");
    halt_forever()
}

// `panic_handler_placeholder()` was removed here. It had zero callers,
// emitted "RIAN PANIC PLACEHOLDER", and halted -- a second, fake panic
// path sitting next to the real one. Anything that genuinely needs to
// stop the kernel calls `halt_forever` directly, after emitting a marker
// that says something true about why.
