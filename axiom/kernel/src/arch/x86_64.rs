pub mod idt;
pub mod port_io;

/// x86_64 architecture support scaffold for ADRIAN OS.

/// Early architecture initialization sequence.
pub fn early_arch_init() {
    early_cpu_init();
    early_descriptor_tables_init();
    early_interrupt_init();
    early_timer_init();
    early_serial_init();
}

fn early_cpu_init() {
    // Planned:
    // - early CPU assumptions
    // - control register policy
    // - CPU-local bootstrap support
}

fn early_descriptor_tables_init() {
    // Confirms the table itself encodes and constructs correctly --
    // every slot starts absent, as it should before any handler is
    // installed. Installing real exception handlers and calling
    // Idt::load() is follow-up work: a loaded IDT needs a `'static`
    // home, which means real kernel-wide static state that doesn't
    // exist yet. GDT layout is also still planned.
    let _idt = idt::Idt::new();
}

fn early_interrupt_init() {
    // Planned:
    // - exception handler stubs
    // - interrupt controller setup
    // - interrupt enable sequencing
}

fn early_timer_init() {
    // Planned:
    // - bootstrap timer source
    // - scheduler tick hook
}

fn early_serial_init() {
    let _ = crate::debug::serial::serial_debug_init();
}
