use crate::boot::BootContext;

/// Main early initialization sequence placeholder.
///
/// This defines the intended initialization order for Axiom.
pub fn early_kernel_init() {
    let boot_context = BootContext::empty();

    if !validate_boot_context(&boot_context) {
        crate::panic::halt_forever();
    }

    early_arch_init();
    early_mm_init();
    early_security_init();
    early_ipc_init();
    early_sched_init();

    enter_idle_placeholder();
}

fn validate_boot_context(context: &BootContext) -> bool {
    context.is_valid()
}

fn early_arch_init() {
    // Future integration point with axiom/arch/x86_64 or arm64 modules.
}

fn early_mm_init() {
    // Future integration point with axiom/mm.
}

fn early_security_init() {
    // Future integration point with axiom/security.
}

fn early_ipc_init() {
    // Future integration point with axiom/ipc.
}

fn early_sched_init() {
    // Future integration point with axiom/sched.
}

fn enter_idle_placeholder() {
    crate::panic::halt_forever();
}
