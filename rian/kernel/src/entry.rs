use crate::boot::BootContext;
use crate::init::InitOutcome;

/// Internal Rian kernel entry boundary.
///
/// Intended conceptual flow:
/// boot-image entry
///   -> boot-image bridge
///   -> boot-image invocation layer
///   -> Rian `kernel_entry(&BootContext)`
///   -> kernel-owned initialization
///
/// In the future, Halo should transfer control through the boot-artifact
/// path that ultimately reaches this boundary.
///
/// Returns the [`InitOutcome`] rather than diverging. This used to
/// `halt_forever()` on an invalid context and then discard init's
/// result, which meant the entry boundary had exactly one observable
/// behavior -- "never comes back" -- and so could not be tested at all.
/// The halt decision belongs to whoever *called* the kernel, because
/// only that caller knows whether it is bare-metal firmware (halt is
/// the only option) or the hosted dev loop (report and exit is far more
/// useful). See [`kernel_entry_and_halt`] for the bare-metal form.
///
/// Note the removed duplicate check: the old body validated the context
/// itself and *then* called init, which validates it again. Two
/// validations with two different failure behaviors is one more than
/// there should be, so this defers to init's, which records the
/// rejection in the boot trace.
pub fn kernel_entry(context: &BootContext) -> InitOutcome {
    crate::init::early_kernel_init_with_context(context)
}

/// The bare-metal entry form: initialize, then never return.
///
/// This is what real firmware jumps to. It exists as a thin wrapper so
/// that the divergence lives in one named place instead of being welded
/// into the middle of the init sequence, where it made every step below
/// it unobservable.
pub fn kernel_entry_and_halt(context: &BootContext) -> ! {
    let outcome = kernel_entry(context);
    if outcome.is_ready() {
        crate::debug::debug_marker("RIAN: INIT COMPLETE, HALTING");
    } else {
        crate::debug::panic_marker(outcome.label());
    }
    crate::panic::halt_forever()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot::BootArchitecture;

    fn valid_context() -> BootContext {
        let mut context = BootContext::empty();
        context.architecture = BootArchitecture::X86_64;
        context
    }

    #[test]
    fn entry_reports_a_successful_init() {
        assert_eq!(kernel_entry(&valid_context()), InitOutcome::Ready);
    }

    #[test]
    fn entry_reports_a_rejected_boot_context_instead_of_hanging() {
        // The behavior this whole refactor bought: the invalid-handoff
        // path is now a value a test can assert on. Previously it was an
        // unconditional `halt_forever()`, i.e. a hang.
        let mut context = valid_context();
        context.header.magic = 0;
        assert_eq!(kernel_entry(&context), InitOutcome::InvalidBootContext);
    }
}
