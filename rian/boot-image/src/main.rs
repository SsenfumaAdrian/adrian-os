//! ADRIAN OS boot-image: host dev-loop wrapper.
//!
//! Constructs a real `BootContext` and crosses into the real Rian
//! kernel entry point (`adrian_kernel::entry::kernel_entry`). There is
//! no bootloader (Halo) and no bootable artifact yet, so this runs as
//! an ordinary hosted program — that's what lets the wrapper -> kernel
//! call path get built and exercised before real firmware and a real
//! bare-metal target exist.
//!
//! This process now *terminates*, with an exit code that reflects
//! whether the kernel initialized. It previously ended in
//! `unreachable!()` behind a kernel that halted forever, which meant the
//! only way to run it was to start it and then Ctrl+C — useless as a CI
//! step. Deciding to halt belongs to bare-metal firmware
//! (`entry::kernel_entry_and_halt`); a hosted dev loop reports and
//! exits.

mod boot_context;

use adrian_kernel::boot_trace::{BootStage, BootTrace, MAX_STAGES};
use adrian_kernel::init::InitOutcome;

fn main() {
    let context = boot_context::host_dev_loop_context();

    println!(
        "adrian-boot-image: constructed BootContext (arch={:?}, valid={})",
        context.architecture,
        context.is_valid()
    );

    if !context.is_valid() {
        eprintln!("adrian-boot-image: BootContext failed validation \u{2014} refusing to cross");
        std::process::exit(2);
    }

    println!("adrian-boot-image: crossing into adrian_kernel::entry::kernel_entry now.");
    let outcome = adrian_kernel::entry::kernel_entry(&context);
    let trace = adrian_kernel::init::boot_trace();

    report(outcome, &trace);
    std::process::exit(exit_code(outcome, &trace));
}

/// Print what boot actually did, in the order it did it.
fn report(outcome: InitOutcome, trace: &BootTrace) {
    print!("adrian-boot-image: boot trace ({}/{}):", trace.len(), MAX_STAGES);
    for stage in trace.stages() {
        print!(" {}", stage.label());
    }
    println!();

    if trace.overflowed() {
        eprintln!("adrian-boot-image: WARNING boot trace overflowed \u{2014} stages were dropped");
    }
    if !trace.is_ordered() {
        eprintln!("adrian-boot-image: WARNING boot stages were recorded out of order");
    }

    let timeouts = adrian_kernel::debug::serial::transmit_timeouts();
    println!(
        "adrian-boot-image: serial transmitter waits that timed out: {timeouts} \
         (expected on a hosted run \u{2014} port I/O is stubbed, so the UART never reports ready)"
    );

    match outcome {
        InitOutcome::Ready => println!("adrian-boot-image: init outcome = ready"),
        other => eprintln!("adrian-boot-image: init outcome = {}", other.label()),
    }
}

/// 0 only for a boot that both reported ready *and* left a complete,
/// ordered trace behind. Two independent checks on purpose: the outcome
/// is init's own opinion of itself, while the trace is evidence of the
/// steps it actually reached, and a disagreement between them is exactly
/// the kind of thing this wrapper exists to catch.
fn exit_code(outcome: InitOutcome, trace: &BootTrace) -> i32 {
    if outcome.is_ready() && trace.is_complete() && trace.last() == Some(BootStage::Idle) {
        0
    } else {
        1
    }
}
