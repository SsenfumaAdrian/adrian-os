//! ADRIAN OS boot-image: host dev-loop wrapper.
//!
//! Constructs a real `BootContext` and crosses into the real Rian
//! kernel entry point (`adrian_kernel::entry::kernel_entry`). There is
//! no bootloader (Halo) and no bootable artifact yet, so this runs as
//! an ordinary hosted program — that's what lets the wrapper -> kernel
//! call path get built and exercised before real firmware and a real
//! bare-metal target exist.

mod boot_context;

use adrian_kernel::boot::BootContext;

fn main() {
    let context = boot_context::host_dev_loop_context();

    println!(
        "adrian-boot-image: constructed BootContext (arch={:?}, valid={})",
        context.architecture,
        context.is_valid()
    );

    if !context.is_valid() {
        eprintln!("adrian-boot-image: BootContext failed validation \u{2014} refusing to cross");
        std::process::exit(1);
    }

    println!("adrian-boot-image: crossing into adrian_kernel::entry::kernel_entry now.");
    println!(
        "adrian-boot-image: this does not return by design \u{2014} a kernel entry point never \
         hands control back. Kernel debug markers follow; Ctrl+C to stop."
    );

    cross_into_kernel(&context)
}

/// `kernel_entry` diverges in practice (every path ends in
/// `halt_forever`), but its signature is `-> ()`, not `-> !` \u{2014} it's
/// written from the kernel's perspective, where nothing is ever there
/// to \"return\" to. This wrapper marks the divergence explicitly at
/// the call site instead of changing the kernel's own signature.
fn cross_into_kernel(context: &BootContext) -> ! {
    adrian_kernel::entry::kernel_entry(context);
    unreachable!("kernel_entry does not return")
}
