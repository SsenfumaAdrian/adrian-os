pub mod idt;
pub mod pic;
pub mod pit;
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
    // Remap first: IRQ0-7 move off the CPU exception range before
    // anything else touches the PICs. Then mask every line -- with no
    // handlers installed yet (every Idt entry from
    // early_descriptor_tables_init is still "missing") and interrupts
    // not yet enabled at the CPU level either (no `sti` anywhere in
    // this codebase), an unhandled interrupt firing would be undefined
    // behavior. Masking is defense in depth alongside that, not a
    // substitute for it.
    pic::PicRemap::standard().apply();
    pic::mask_all();
}

fn early_timer_init() {
    // Programming a rate is safe on its own: PIT ticks arrive on IRQ0,
    // which early_interrupt_init() already masked, and the CPU-level
    // interrupt flag is still off regardless. Hooking this into an
    // actual scheduler tick is follow-up work, once there's a
    // scheduler to tick.
    if let Some(divisor) = pit::divisor_for_frequency(1000) {
        pit::program_channel0(divisor);
    }
}

fn early_serial_init() {
    let _ = crate::debug::serial::serial_debug_init();
}
