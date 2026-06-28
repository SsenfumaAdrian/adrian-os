use crate::boot::BootContext;

/// Main early initialization sequence placeholder.
///
/// This defines the intended initialization order for Axiom.
pub fn early_kernel_init() {
    let boot_context = BootContext::empty();

    if !validate_boot_context(&boot_context) {
        crate::panic::halt_forever();
    }

    crate::arch::early_arch_init();
    crate::mm::early_mm_init();
    crate::security::early_security_init();
    crate::ipc::early_ipc_init();
    crate::sched::early_sched_init();

    enter_idle_placeholder();
}

fn validate_boot_context(context: &BootContext) -> bool {
    context.is_valid()
}

fn enter_idle_placeholder() {
    crate::panic::halt_forever();
}
