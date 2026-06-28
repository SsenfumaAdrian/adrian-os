use crate::boot::BootContext;

/// Internal Axiom kernel entry boundary.
///
/// In the future, Halo should transfer control here with a validated
/// BootContext-like structure. This function then routes into the
/// generic early initialization flow.
pub fn kernel_entry(context: &BootContext) {
    if !context.is_valid() {
        crate::panic::halt_forever();
    }

    crate::init::early_kernel_init_with_context(context);
}
