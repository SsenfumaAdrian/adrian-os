use crate::boot::BootContext;

/// Legacy top-level init path used by current scaffolding.
pub fn early_kernel_init() {
    let boot_context = BootContext::empty();
    early_kernel_init_with_context(&boot_context);
}

/// Main early initialization sequence with explicit boot context.
pub fn early_kernel_init_with_context(context: &BootContext) {
    if !validate_boot_context(context) {
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
