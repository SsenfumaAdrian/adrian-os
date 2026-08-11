//! Constructs the real `adrian_kernel::boot::BootContext` this wrapper
//! hands to the kernel.
//!
//! There is no bootloader (Halo) behind this yet, so there is no real
//! memory map and no real framebuffer. Those fields stay at the zeroed
//! defaults from `BootContext::empty()` rather than being filled with
//! values that would look real but aren't — once Halo exists and reads
//! actual firmware data, it becomes the thing constructing this
//! context, and this function goes away.

use adrian_kernel::boot::{BootArchitecture, BootContext};

/// Build the BootContext for a host dev-loop run.
pub fn host_dev_loop_context() -> BootContext {
    let mut context = BootContext::empty();
    context.architecture = BootArchitecture::X86_64;
    context
}
