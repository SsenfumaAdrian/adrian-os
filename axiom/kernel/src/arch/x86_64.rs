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
    // Planned:
    // - GDT layout
    // - IDT layout
    // - privilege transition planning
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
    // Planned:
    // - initialize earliest serial debug path
    // - prepare QEMU-visible bring-up diagnostics
}
