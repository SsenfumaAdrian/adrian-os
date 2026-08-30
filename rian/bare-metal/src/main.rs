//! The bare-metal boot artifact.
//!
//! This crate is the ELF that a bootloader loads and jumps into. It owns
//! exactly three things and deliberately nothing else: the entry stub in
//! `boot.s`, the translation from a bootloader's handoff convention into
//! the kernel's [`BootContext`], and the call into the kernel. Anything
//! that looks like kernel logic belongs in `adrian-kernel`, where it can
//! be unit tested; anything here can only be verified by booting.
//!
//! Not to be confused with `rian/boot-image`, which despite the name is
//! the *hosted* dev loop -- a normal `std` binary that calls the same
//! kernel entry point from a userspace process so the init sequence can be
//! exercised without hardware. The two are complementary: `boot-image`
//! answers "does init run", this crate answers "does it run in ring 0".
#![no_std]
#![no_main]

use adrian_kernel::boot::{BootArchitecture, BootContext};
use adrian_kernel::debug;
use adrian_kernel::entry::kernel_entry_and_halt;

// The entry stub. `include_str!` rather than a separate assembly build
// step so the whole image builds with `cargo build` and no external
// assembler -- see the header comment in boot.s for why that constraint
// exists. No `options(raw)`: boot.s contains no braces, which is an
// invariant stated at the top of that file.
core::arch::global_asm!(include_str!("boot.s"));

/// What a multiboot1-compliant loader leaves in `eax` before jumping to
/// the image. Checked rather than assumed, because the same entry point
/// is reachable from anything that can load an ELF, and a loader that did
/// not follow multiboot has also not set up the state this image expects.
const MULTIBOOT1_BOOTLOADER_MAGIC: u32 = 0x2BAD_B002;

/// First Rust code to run in ring 0.
///
/// Called from `long_mode_entry` in boot.s with the multiboot handoff in
/// the SysV argument registers. By the time this runs: the bss is zeroed,
/// a 64 KiB stack is live, the first 1 GiB is identity-mapped with 2 MiB
/// pages, and a flat 64-bit GDT is loaded. There is still no IDT, no TSS
/// and no exception handler, so any fault from here is a triple fault --
/// that is roadmap step 3, and it is the next thing that should exist.
///
/// `extern "C"` and `#[no_mangle]` because the assembly calls it by name.
#[no_mangle]
pub extern "C" fn rian_main(bootloader_magic: u32, bootloader_info: u64) -> ! {
    // Bring the UART up before saying anything through it. `init` calls
    // this too, and calling it twice is harmless -- it is nine register
    // writes with no state of its own -- but the handoff report below
    // happens before init starts, and an unconfigured UART would emit it
    // at whatever divisor the firmware left behind.
    debug::serial::serial_debug_init();

    debug::debug_marker(handoff_label(bootloader_magic, bootloader_info));

    kernel_entry_and_halt(&bare_metal_boot_context())
}

/// Describe the handoff without acting on it.
///
/// Reported rather than used, and that is the honest state of things: the
/// multiboot information structure at `bootloader_info` carries the memory
/// map that roadmap step 4 needs, but parsing it is that step's work.
/// Today the only true thing to say is whether the handoff had the shape
/// the image was built for -- and saying it is what makes the difference
/// between "booted" and "booted from a loader we understand" visible in
/// the serial log rather than assumed.
///
/// Split out from [`rian_main`] as a pure function over the two handoff
/// values so it has a return value something could assert on. Nothing
/// asserts on it yet: this crate has no tests, because `cargo test` would
/// have to build it for the host, and it does not link for the host.
const fn handoff_label(bootloader_magic: u32, bootloader_info: u64) -> &'static str {
    if bootloader_magic != MULTIBOOT1_BOOTLOADER_MAGIC {
        // Reached the kernel, but not the way the image was designed to
        // be reached. Continuing anyway is the right call: the entry stub
        // has already established every precondition the kernel actually
        // depends on, so the only thing lost is the memory map.
        "RIAN: handoff magic unrecognized, continuing without a memory map"
    } else if bootloader_info == 0 {
        "RIAN: multiboot1 handoff with no information structure"
    } else {
        "RIAN: multiboot1 handoff"
    }
}

/// The `BootContext` a bare-metal boot hands the kernel.
///
/// Every field it does not fill is a subsystem that does not exist yet,
/// and each one is left at its `empty()` value rather than being given a
/// plausible-looking number:
///
/// * `memory_map.entry_count` stays 0 -- the multiboot map is not parsed,
///   so the allocator is still seeded with nothing. Roadmap step 4.
/// * `framebuffer` stays zeroed -- the header does not request a mode, so
///   the loader provides none.
/// * `flags` stays `NONE` -- `BootFlags` has no defined bits, and
///   inventing one here would put the definition in the caller.
fn bare_metal_boot_context() -> BootContext {
    let mut context = BootContext::empty();
    context.architecture = BootArchitecture::X86_64;
    context
}
