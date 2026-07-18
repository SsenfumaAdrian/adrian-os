use crate::boot::BootContext;

/// Legacy top-level init path used by current scaffolding.
pub fn early_kernel_init() {
    let boot_context = BootContext::empty();
    early_kernel_init_with_context(&boot_context);
}

/// Main early initialization sequence with explicit boot context.
pub fn early_kernel_init_with_context(context: &BootContext) {
    crate::debug::serial::serial_debug_init();
    crate::debug::debug_marker("AXIOM: ENTRY");

    if !validate_boot_context(context) {
        crate::debug::panic_marker("AXIOM: INVALID BOOT CONTEXT");
        crate::panic::halt_forever();
    }

    crate::debug::debug_marker("AXIOM: BOOT CONTEXT OK");

    crate::debug::debug_marker("AXIOM: ARCH INIT");
    crate::arch::early_arch_init();

    crate::debug::debug_marker("AXIOM: MM INIT");
    // No bootloader (Halo) yet, so there is no real memory map to pass --
    // an empty slice is honest about that. Seeds the global bootstrap
    // allocator (mm::BOOTSTRAP_ALLOCATOR); other subsystems can now
    // allocate from it directly rather than threading it through by hand.
    crate::mm::early_mm_init(&[]);

    crate::debug::debug_marker("AXIOM: SECURITY INIT");
    crate::security::early_security_init();

    crate::debug::debug_marker("AXIOM: IPC INIT");
    crate::ipc::early_ipc_init();

    crate::debug::debug_marker("AXIOM: SCHED INIT");
    crate::sched::early_sched_init();

    crate::debug::debug_marker("AXIOM: PROCESS INIT");
    let kernel_process_id = crate::process::early_process_init();

    crate::debug::debug_marker("AXIOM: THREAD INIT");
    crate::thread::early_thread_init(kernel_process_id);

    crate::debug::debug_marker("AXIOM: HALT");
    enter_idle_placeholder();
}

fn validate_boot_context(context: &BootContext) -> bool {
    context.is_valid()
}

fn enter_idle_placeholder() {
    crate::panic::halt_forever();
}
