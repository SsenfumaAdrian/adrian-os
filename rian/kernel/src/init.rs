use crate::boot::BootContext;
use crate::boot_trace::{self, BootStage, BootTrace};

// `early_kernel_init()` -- a no-argument wrapper that built a
// `BootContext::empty()` and called into the sequence below -- was removed
// alongside `lib.rs`'s `kernel_init()`, its only caller. It described
// itself as the "legacy top-level init path used by current scaffolding",
// and that scaffolding is gone. `entry::kernel_entry` supplies a real
// `BootContext`, so the context-taking form below is the only init path.

/// Why initialization stopped.
///
/// Init used to end by calling `halt_forever()` itself, which made the
/// whole sequence untestable: there is no way to assert anything about a
/// function that never returns. Deciding *what to do once the kernel is
/// initialized* is not initialization's job -- it belongs to the entry
/// point, which is the only code that knows whether it is running on
/// bare metal (halt) or in the hosted dev loop (report and exit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitOutcome {
    /// Every subsystem initialized. The kernel is ready to idle.
    Ready,
    /// The bootloader handed over a `BootContext` that failed
    /// validation. Nothing was initialized; nothing can be trusted.
    InvalidBootContext,
    /// The kernel's own bootstrap process could not be created, so
    /// there is no process for the first kernel thread to live in.
    /// Reachable only if the process table is exhausted before boot's
    /// own spawn, which nothing does today -- reported rather than
    /// panicked on because a kernel that cannot start should say why.
    ProcessInitFailed,
    /// The bootstrap process exists but its first thread could not be
    /// created, so the scheduler's ready queue would be empty and
    /// there would be nothing to run. Same reachability story as
    /// `ProcessInitFailed`: only on an exhausted thread table.
    ThreadInitFailed,
}

impl InitOutcome {
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// A short, stable identifier for logs. Same reasoning as
    /// `BootStage::label`: `no_std` boot paths have no formatting
    /// machinery, so a caller that wants to report the outcome needs a
    /// plain `&'static str` rather than `{:?}`.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::InvalidBootContext => "invalid-boot-context",
            Self::ProcessInitFailed => "process-init-failed",
            Self::ThreadInitFailed => "thread-init-failed",
        }
    }
}

/// Main early initialization sequence with explicit boot context.
///
/// Records each step into the global [`boot_trace::BOOT_TRACE`] as it is
/// reached, so how far boot got becomes a value that can be read back
/// rather than text that scrolled past on a serial line.
pub fn early_kernel_init_with_context(context: &BootContext) -> InitOutcome {
    run_init(context, boot_trace::record)
}

/// The init sequence itself, with progress reporting injected.
///
/// Taking the recorder as a parameter rather than writing straight to
/// the global is what makes this testable: `cargo test` runs tests in
/// parallel threads inside one process, so several tests driving one
/// global boot record would interleave and flake. Tests pass a closure
/// over their own local `BootTrace`; production passes
/// `boot_trace::record`. Same code path either way -- the test is not
/// exercising a special-cased variant of boot.
fn run_init<R: FnMut(BootStage)>(context: &BootContext, mut record: R) -> InitOutcome {
    crate::debug::serial::serial_debug_init();
    record(BootStage::Entry);

    if !validate_boot_context(context) {
        crate::debug::panic_marker("RIAN: INVALID BOOT CONTEXT");
        return InitOutcome::InvalidBootContext;
    }
    record(BootStage::BootContextValidated);

    record(BootStage::ArchInit);
    crate::arch::early_arch_init();

    record(BootStage::MemoryInit);
    // No bootloader (Halo) yet, so there is no real memory map to pass --
    // an empty slice is honest about that. Seeds the global bootstrap
    // allocator (mm::BOOTSTRAP_ALLOCATOR); other subsystems can now
    // allocate from it directly rather than threading it through by hand.
    crate::mm::early_mm_init(&[]);

    record(BootStage::SecurityInit);
    crate::security::early_security_init();

    record(BootStage::IpcInit);
    crate::ipc::early_ipc_init();

    record(BootStage::SchedulerInit);
    crate::sched::early_sched_init();

    record(BootStage::ProcessInit);
    let kernel_process_id = match crate::process::early_process_init() {
        Ok(id) => id,
        Err(_) => {
            crate::debug::panic_marker("RIAN: BOOTSTRAP PROCESS CREATION FAILED");
            return InitOutcome::ProcessInitFailed;
        }
    };

    record(BootStage::ThreadInit);
    // The bootstrap thread is what makes the ready queue non-empty. If
    // it cannot be created there is nothing for the scheduler to pick,
    // so this is a boot failure and not a degraded-but-running state.
    if crate::thread::early_thread_init(kernel_process_id).is_err() {
        crate::debug::panic_marker("RIAN: BOOTSTRAP THREAD CREATION FAILED");
        return InitOutcome::ThreadInitFailed;
    }

    record(BootStage::Idle);
    InitOutcome::Ready
}

/// The global boot trace as it stands. Convenience accessor so callers
/// of `early_kernel_init_with_context` need not reach into `boot_trace`
/// separately to find out what happened.
pub fn boot_trace() -> BootTrace {
    boot_trace::snapshot()
}

fn validate_boot_context(context: &BootContext) -> bool {
    context.is_valid()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot::BootArchitecture;
    use crate::boot_trace::MAX_STAGES;

    /// A context that passes validation, the way a real bootloader
    /// eventually will.
    fn valid_context() -> BootContext {
        let mut context = BootContext::empty();
        context.architecture = BootArchitecture::X86_64;
        context
    }

    /// Run the real init sequence, capturing progress locally.
    fn boot(context: &BootContext) -> (InitOutcome, BootTrace) {
        let mut trace = BootTrace::new();
        let outcome = run_init(context, |stage| {
            trace.record(stage);
        });
        (outcome, trace)
    }

    /// The first test in this project that exercises the *whole* boot
    /// sequence rather than one subsystem in isolation. Nine init
    /// functions were previously reachable only through a path that
    /// ended in an infinite loop, so nothing asserted they ran at all.
    #[test]
    fn a_valid_boot_context_initializes_every_subsystem_in_order() {
        let (outcome, trace) = boot(&valid_context());

        assert_eq!(outcome, InitOutcome::Ready);
        assert!(outcome.is_ready());
        assert!(
            trace.is_complete(),
            "boot reached {} of {} stages, last was {:?}",
            trace.len(),
            MAX_STAGES,
            trace.last()
        );
        assert!(trace.is_ordered());
        assert!(!trace.overflowed());
        assert_eq!(trace.last(), Some(BootStage::Idle));

        // Name three stages explicitly as well. `is_ordered` only
        // checks the shape of what was recorded; these check that the
        // steps we care about are the ones that actually ran.
        assert!(trace.reached(BootStage::Entry));
        assert!(trace.reached(BootStage::MemoryInit));
        assert!(trace.reached(BootStage::ThreadInit));
    }

    #[test]
    fn an_invalid_boot_context_stops_init_before_touching_any_subsystem() {
        let mut context = BootContext::empty();
        context.header.magic = 0;
        assert!(!context.is_valid());

        let (outcome, trace) = boot(&context);

        assert_eq!(outcome, InitOutcome::InvalidBootContext);
        assert!(!outcome.is_ready());
        // Entry only: reached the kernel, rejected the handoff. If this
        // ever grows a second stage, init started work on a context it
        // had already decided not to trust.
        assert_eq!(trace.len(), 1);
        assert_eq!(trace.last(), Some(BootStage::Entry));
        assert!(!trace.reached(BootStage::BootContextValidated));
        assert!(!trace.is_complete());
    }

    #[test]
    fn a_wrong_boot_context_version_is_also_rejected() {
        // Magic right, version wrong: what a future v2 bootloader
        // handing over to a v1 kernel would hit. Checked separately
        // from the bad-magic case because they are different failures
        // and `is_valid` tests both conditions.
        let mut context = valid_context();
        context.header.version = BootContext::VERSION + 1;

        let (outcome, trace) = boot(&context);
        assert_eq!(outcome, InitOutcome::InvalidBootContext);
        assert!(!trace.reached(BootStage::BootContextValidated));
    }

    #[test]
    fn init_is_idempotent_enough_to_run_twice() {
        // Not a claim that re-initializing a live kernel is sensible --
        // it is not. This pins something narrower and genuinely useful:
        // no init step panics or wedges when the global tables it
        // touches are already populated, which is exactly the state the
        // second run finds them in. Without this, the subsystem globals
        // would make test ordering load-bearing.
        let context = valid_context();
        let (first, first_trace) = boot(&context);
        let (second, second_trace) = boot(&context);

        assert_eq!(first, InitOutcome::Ready);
        assert_eq!(second, InitOutcome::Ready, "second boot must also complete");
        assert!(first_trace.is_complete());
        assert!(second_trace.is_complete());
    }
}
