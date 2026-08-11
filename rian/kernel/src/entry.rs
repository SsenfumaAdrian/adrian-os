use crate::boot::BootContext;

/// Internal Rian kernel entry boundary.
///
/// Future conceptual caller:
/// - boot-image wrapper invocation layer
///
/// Intended conceptual flow:
/// boot-image entry
///   -> boot-image bridge
///   -> boot-image invocation layer
///   -> Rian kernel_entry(&BootContext)
///   -> kernel-owned initialization
///
/// In the future, Halo should transfer control through the boot-artifact
/// path that ultimately reaches this boundary.
pub fn kernel_entry(context: &BootContext) {
    if !context.is_valid() {
        crate::panic::halt_forever();
    }

    crate::init::early_kernel_init_with_context(context);
}
